extern crate hyper;
extern crate url;

use self::hyper::Client;
use self::hyper::header::{Connection, ContentLength, Referer};

use std::io::Read;
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

use super::{Progress, RomConfig};

trait FindFrom {
    fn find_from(&self, pat: &str, start: usize) -> Option<usize>;
}

impl FindFrom for str {
    fn find_from(&self, pat: &str, start: usize) -> Option<usize> {
        if let Some(pos) = self[start .. ].find(pat) {
            Some(start + pos)
        } else {
            None
        }
    }
}

trait FindSkip {
    fn find_skip(&self, pat: &str) -> Option<usize>;
}

impl FindSkip for str {
    fn find_skip(&self, pat: &str) -> Option<usize> {
        if let Some(pos) = self.find(pat) {
            Some(pos + pat.len())
        } else {
            None
        }
    }
}

trait FindFromSkip {
    fn find_from_skip(&self, pat: &str, start: usize) -> Option<usize>;
}

impl FindFromSkip for str {
    fn find_from_skip(&self, pat: &str, start: usize) -> Option<usize> {
        if let Some(pos) = self[start .. ].find(pat) {
            Some(start + pos + pat.len())
        } else {
            None
        }
    }
}

#[derive(Debug, Default)]
struct Page {
    count: usize,
    index: usize,
    total: usize,
}

fn parse(html: &str, roms: &mut Vec<RomConfig>) -> Page {
    let mut page = Page::default();

    for line in html.lines() {
        if line.find("<meta name=\"description\" content=\"Now listing roms for ").is_some() {
            if let Some(p) = line.find_skip("Showing ") {
                if let Some(n) = line.find_from(" ", p) {
                    if let Ok(count) = usize::from_str_radix(&line[p .. n].replace(",", ""), 10) {
                        page.count = count;
                    }
                }

                if let Some(p) = line.find_from_skip("index ", p) {
                    if let Some(n) = line.find_from(" ", p) {
                        if let Ok(index) = usize::from_str_radix(&line[p .. n].replace(",", ""), 10) {
                            page.index = index;
                        }
                    }

                    if let Some(p) = line.find_from_skip("of ", p) {
                        if let Some(n) = line.find_from(" ", p) {
                            if let Ok(total) = usize::from_str_radix(&line[p .. n].replace(",", ""), 10) {
                                page.total = total;
                            }
                        }
                    }
                }
            }
        }

        if line.find("<td height=\"40\" align=\"left\" valign=\"middle\"><a id=\"listing\" ").is_some() {
            let mut entry = RomConfig::default();

            if let Some(p) = line.find_skip("name=\"") {
                if let Some(n) = line.find_from("\" ", p) {
                    entry.file = line[p .. n].to_string();
                }

                if let Some(p) = line.find_from_skip("<img src=\\'", p) {
                    if let Some(n) = line.find_from("\\' ", p) {
                        entry.image = line[p .. n].to_string();
                    }

                    if let Some(p) = line.find_from_skip("<b>Game Name</b>:</font> </td><td valign=top align=left><font size=-2>", p) {
                        if let Some(n) = line.find_from(" </font>", p) {
                            entry.name = line[p .. n].to_string();
                        }
                    }
                }
            }

            if entry.file != "No Roms" {
                roms.push(entry);
            }
        }
    }

    page
}

pub struct List {
    progress: Arc<Mutex<Progress>>,
    result: JoinHandle<Vec<RomConfig>>,
}

impl List {
    pub fn new(system: &str) -> List {
        let progress = Arc::new(Mutex::new(Progress::Connecting));
        let progress_child = progress.clone();
        let system_child = system.to_string();

        let result = thread::spawn(move || -> Vec<RomConfig> {
            let mut roms = Vec::new();

            let client = Client::new();
            let mut next_index = 0;
            'downloading: loop {
                match client
                    .get(&format!("http://doperoms.com/roms/{}/{}.html", system_child, next_index))
                    .header(Connection::keep_alive())
                    .header(Referer("http://www.doperoms.com/".to_string()))
                    .send()
                {
                    Ok(mut res) => {
                        let mut html = String::new();
                        match res.read_to_string(&mut html) {
                            Ok(_) => {
                                let page = parse(&html, &mut roms);

                                if let Ok(mut progress) = progress_child.lock() {
                                    *progress = Progress::InProgress(page.index as u64, page.total as u64);
                                }

                                next_index = page.index + page.count;
                                if next_index >= page.total {
                                    if let Ok(mut progress) = progress_child.lock() {
                                        *progress = Progress::Complete;
                                    }
                                    break 'downloading;
                                }
                            },
                            Err(err) => {
                                if let Ok(mut progress) = progress_child.lock() {
                                    *progress = Progress::Error(format!("{:?}", err));
                                }
                                break 'downloading;
                            }
                        }
                    },
                    Err(err) => {
                        if let Ok(mut progress) = progress_child.lock() {
                            *progress = Progress::Error(format!("{:?}", err));
                        }
                        break 'downloading;
                    }
                }
            }

            roms
        });

        List {
            progress: progress,
            result: result,
        }
    }

    pub fn progress(&self) -> Progress {
        match self.progress.lock() {
            Ok(progress) => progress.clone(),
            Err(err) => Progress::Error(format!("{:?}", err))
        }
    }

    pub fn result(self) -> Vec<RomConfig> {
        if let Some(roms) = self.result.join().ok() {
            roms
        } else {
            Vec::new()
        }
    }
}

pub struct Download {
    progress: Arc<Mutex<Progress>>
}

impl Download {
    pub fn new(system: &str, file: &str) -> Download {
        let progress = Arc::new(Mutex::new(Progress::Connecting));
        let progress_child = progress.clone();
        let url_child = format!("http://doperoms.com/files/roms/{}/GETFILE_{}", system, url::percent_encoding::utf8_percent_encode(file, url::percent_encoding::DEFAULT_ENCODE_SET));

        thread::spawn(move || {
            match Client::new()
                    .get(&url_child)
                    .header(Connection::keep_alive())
                    .header(Referer("http://doperoms.com/".to_string()))
                    .send()
            {
                Ok(mut res) => {
                    if let Some(&ContentLength(total)) = res.headers.get() {
                        let mut downloaded = 0;
                        if let Ok(mut progress) = progress_child.lock() {
                            *progress = Progress::InProgress(downloaded, total);
                        }

                        let mut bytes = [0; 65536];
                        'reading: loop {
                            match res.read(&mut bytes) {
                                Ok(0) => {
                                    if let Ok(mut progress) = progress_child.lock() {
                                        *progress = Progress::Complete;
                                    }
                                    break 'reading;
                                }
                                Ok(count) => {
                                    downloaded += count as u64;
                                    if let Ok(mut progress) = progress_child.lock() {
                                        *progress = Progress::InProgress(downloaded, total);
                                    }
                                }
                                Err(err) => {
                                    if let Ok(mut progress) = progress_child.lock() {
                                        *progress = Progress::Error(format!("{:?}", err));
                                    }
                                    break 'reading;
                                }
                            }
                        }
                    } else {
                        if let Ok(mut progress) = progress_child.lock() {
                            *progress = Progress::Error("No ContentLength".to_string());
                        }
                    }
                },
                Err(err) => if let Ok(mut progress) = progress_child.lock() {
                    *progress = Progress::Error(format!("{:?}", err));
                }
            }
        });

        Download {
            progress: progress
        }
    }

    pub fn progress(&self) -> Progress {
        match self.progress.lock() {
            Ok(progress) => progress.clone(),
            Err(err) => Progress::Error(format!("{:?}", err))
        }
    }
}
