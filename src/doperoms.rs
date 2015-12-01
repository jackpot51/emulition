use std::process::Command;

use super::RomConfig;

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
struct Entry {
    file: String,
    image: String,
    name: String,
}

#[derive(Debug, Default)]
struct Page {
    count: usize,
    index: usize,
    total: usize,
    entries: Vec<Entry>,
}

fn parse(html: &str) -> Page {
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
            let mut entry = Entry::default();

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

            page.entries.push(entry);
        }
    }

    page
}

pub fn list(system: &str) -> Vec<RomConfig> {
    let mut roms = Vec::new();

    let mut next_index = 0;
    loop {
        let output = Command::new("wget")
                        .arg("-O")
                        .arg("-")
                        .arg(&format!("http://www.doperoms.com/roms/{}/{}.html", system, next_index))
                        .output().unwrap();

        let page = parse(&String::from_utf8_lossy(&output.stdout));

        for entry in page.entries.iter() {
            println!("{:?}", entry);

            roms.push(RomConfig {
                name: entry.name.clone(),
                file: entry.file.clone(),
                image: entry.image.clone(),
            });
        }

        next_index = page.index + page.count;
        if next_index >= page.total {
            break;
        }
    }

    return roms;
}
