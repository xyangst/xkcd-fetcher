use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::to_string_pretty;
use std::collections::BTreeMap;
use std::fs::{File, OpenOptions};
use std::io::{BufReader, Write};
use std::path::Path;
use ureq::get;

type Comics = BTreeMap<usize, Comic>;
fn main() -> anyhow::Result<()> {
    let amount_of_comics = Comic::fetch(0)?.unwrap().num;
    //once we have all the metadata for comics, we can fetch the actual comics
    let comics: BTreeMap<usize, Comic> = File::open("comics/meta.json")
        .context("meta.json doesnt exist")
        .and_then(|file| {
            serde_json::from_reader(file)
                .context("Failed to deserialize")
                .and_then(|mut f: Comics| {
                    let last_key = *f.last_key_value().unwrap().0;
                    if last_key != amount_of_comics {
                        println!("fetching {} new comics!", amount_of_comics - last_key);
                        let comics = Comic::fetch_multiple(last_key..amount_of_comics);
                        f.append(&mut comics?);
                    }
                    write_comics_to_disk(&f);
                    Ok(f)
                })
        })
        .or_else(|_| {
            eprintln!("Error occurred, fetching instead...");
            let c = Comic::fetch_multiple(1..amount_of_comics).context("failed to fetch")?;
            write_comics_to_disk(&c);
            Ok::<_,anyhow::Error>(c)
        })?;

    for (_, comic) in comics {
        let path = comic.get_image_path();
        if !Path::new(&path).exists() {
            let mut file = File::create(&path)?;
            println!("fetching image for:{}", comic.title);
            let mut reader = get(&comic.img).call()?.into_reader();
            std::io::copy(&mut reader, &mut file)?;
        }
    }
    Ok(())
}

#[derive(Deserialize, Serialize, Debug)]
struct Comic {
    month: String,
    link: String,
    year: String,
    news: String,
    num: usize,
    safe_title: String,
    transcript: String,
    alt: String,
    img: String,
    title: String,
    day: String,
}
impl Comic {
    fn get_image_path(&self) -> String {
        let path = self.img.rsplit_once('/').unwrap().1;
        format!("comics/{path}")
    }
    fn fetch(id: usize) -> anyhow::Result<Option<Self>> {
        if id == 404 {
            return Ok(None);
        }
        let url = if id == 0 {
            format!("https://xkcd.com/info.0.json")
        } else {
            format!("https://xkcd.com/{id}/info.0.json")
        };
        let res = get(&url).call()?.into_json()?;
        Ok(Some(res))
    }
    fn fetch_multiple(ids: std::ops::Range<usize>) -> Result<BTreeMap<usize, Comic>> {
        let mut comics = BTreeMap::new();
        for id in ids {
            let Some(comic) = Comic::fetch(id)? else {
                println!("ignoring comic number {id}");
                continue;
            };
            comics.insert(id, comic);
            println!("fetched comic number,{}", id);
        }

        Ok(comics)
    }
}

fn write_comics_to_disk(c: &Comics) -> Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open("meta.json")?;
    serde_json::to_writer(file, c)?;
    Ok(())
}
