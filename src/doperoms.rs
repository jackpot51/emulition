extern crate reqwest;
extern crate url;

use self::reqwest::ClientBuilder;
use self::reqwest::header::{CONNECTION, CONTENT_LENGTH, REFERER};

use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

use rom::{Progress, RomConfig, RomFlags};

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

    let mut entry = RomConfig::default();
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

        if line.find("<td height=\"40\" align=\"left\" valign=\"middle\" nowrap=\"nowrap\">").is_some() {
            if let Some(p) = line.find_skip("<img src=\"https://www.doperoms.com/") {
                if let Some(n) = line.find_from(".gif\" ", p) {
                    let flag = line[p .. n].to_string();
                    if flag == "good" {
                        entry.flags.push(RomFlags::Good);
                    } else if flag == "cracked" {
                        entry.flags.push(RomFlags::Cracked);
                    } else if flag == "alternate" {
                        entry.flags.push(RomFlags::Alternate);
                    } else if flag == "trainer" {
                        entry.flags.push(RomFlags::Trainer);
                    } else if flag == "fix" {
                        entry.flags.push(RomFlags::Fix);
                    } else if flag == "hack" {
                        entry.flags.push(RomFlags::Hack);
                    } else if flag == "publicdomain" {
                        entry.flags.push(RomFlags::PublicDomain);
                    } else if flag == "bad" {
                        entry.flags.push(RomFlags::Bad);
                    } else if flag == "overdump" {
                        entry.flags.push(RomFlags::OverDump);
                    }
                }
            }
        }

        if line.find("<td height=\"40\" align=\"left\" valign=\"middle\"><a id=\"listing\" ").is_some() {
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
                entry = RomConfig::default();
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

            let client = ClientBuilder::new()
                .gzip(false)
                .build().unwrap();
            let mut next_index = 0;
            'downloading: loop {
                match client
                    .get(&format!("https://doperoms.com/roms/{}/ALL/{}.html", system_child, next_index))
                    .header(CONNECTION, "keep-alive")
                    .header(REFERER, "https://www.doperoms.com/")
                    .send()
                    .and_then(|x| x.error_for_status())
                {
                    Ok(mut res) => {
                        match res.text() {
                            Ok(html) => {
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
                                    println!("list res.text: {}", err);
                                    *progress = Progress::Error(format!("{}", err));
                                }
                                break 'downloading;
                            }
                        }
                    },
                    Err(err) => {
                        if let Ok(mut progress) = progress_child.lock() {
                            println!("list client send: {}", err);
                            *progress = Progress::Error(format!("{}", err));
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
            Err(err) => Progress::Error(format!("{}", err))
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
    progress: Arc<Mutex<Progress>>,
    result: JoinHandle<()>
}

impl Download {
    pub fn new(url: &str, path: &Path) -> Download {
        let progress = Arc::new(Mutex::new(Progress::Connecting));
        let progress_child = progress.clone();
        let url_child = url.to_string();
        let path_child = path.to_path_buf();

        let result = thread::spawn(move || {
            if let Some(parent) = path_child.parent() {
                if ! parent.is_dir() {
                    match fs::create_dir_all(parent) {
                        Ok(_) => (),
                        Err(err) => {
                            println!("create dir: {}", err);
                            if let Ok(mut progress) = progress_child.lock() {
                                *progress = Progress::Error(format!("{}", err));
                            }
                            return;
                        }
                    }
                }
            }

            match File::create(&path_child) {
                Ok(mut file) => {
                    match ClientBuilder::new()
                            .build().unwrap()
                            .get(&url_child)
                            .header(CONNECTION, "keep-alive")
                            .header(REFERER, "https://doperoms.com/")
                            .send()
                    {
                        Ok(mut res) => {
                            if let Some(total) = res.headers()
                                .get(CONTENT_LENGTH)
                                .and_then(|x| x.to_str().ok())
                                .and_then(|x| x.parse().ok())
                            {
                                let mut downloaded = 0;
                                if let Ok(mut progress) = progress_child.lock() {
                                    *progress = Progress::InProgress(downloaded, total);
                                }

                                let mut bytes = [0; 4096];
                                'downloading: loop {
                                    match res.read(&mut bytes) {
                                        Ok(0) => {
                                            if let Ok(mut progress) = progress_child.lock() {
                                                *progress = Progress::Complete;
                                            }
                                            break 'downloading;
                                        }
                                        Ok(count) => {
                                            match file.write(&bytes[ .. count]) {
                                                Ok(_) => {
                                                    downloaded += count as u64;
                                                    if let Ok(mut progress) = progress_child.lock() {
                                                        *progress = Progress::InProgress(downloaded, total);
                                                    }
                                                },
                                                Err(err) => {
                                                    println!("file write: {}", err);
                                                    if let Ok(mut progress) = progress_child.lock() {
                                                        *progress = Progress::Error(format!("{}", err));
                                                    }
                                                    break 'downloading;
                                                }
                                            }
                                        }
                                        Err(err) => {
                                            println!("res read: {}", err);
                                            if let Ok(mut progress) = progress_child.lock() {
                                                *progress = Progress::Error(format!("{}", err));
                                            }
                                            break 'downloading;
                                        }
                                    }
                                }
                            } else {
                                println!("no content length");
                                if let Ok(mut progress) = progress_child.lock() {
                                    *progress = Progress::Error("No ContentLength".to_string());
                                }
                            }
                        },
                        Err(err) => {
                            println!("client send: {}", err);
                            if let Ok(mut progress) = progress_child.lock() {
                                *progress = Progress::Error(format!("{}", err));
                            }
                        }
                    }
                },
                Err(err) => {
                    println!("file open: {}", err);
                    if let Ok(mut progress) = progress_child.lock() {
                        *progress = Progress::Error(format!("{}", err));
                    }
                }
            }
        });

        Download {
            progress: progress,
            result: result
        }
    }

    pub fn rom(system: &str, rom: &str, path: &Path) -> Download {
        Download::new(
            &format!("https://doperoms.com/files/roms/{}/GETFILE_{}", system, url::percent_encoding::utf8_percent_encode(rom, url::percent_encoding::DEFAULT_ENCODE_SET)),
            path
        )
    }

    pub fn progress(&self) -> Progress {
        match self.progress.lock() {
            Ok(progress) => progress.clone(),
            Err(err) => Progress::Error(format!("{}", err))
        }
    }

    pub fn result(self) -> bool {
        match self.result.join() {
            Ok(_) => true,
            Err(_) => false
        }
    }
}
