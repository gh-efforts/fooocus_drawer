use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{anyhow, Result};
use base64::Engine;
use base64::engine::general_purpose;
use clap::Parser;
use image::{DynamicImage, ImageFormat};
use notify::{EventKind, RecursiveMode, Watcher};
use serde::Deserialize;
use show_image::create_window;
use ureq::Agent;

fn image_prompt(
    img: &Path,
    client: &Agent,
    api: &str,
) -> Result<DynamicImage> {
    let encoded = general_purpose::STANDARD.encode(std::fs::read(img)?);

    let resp = client.post(api)
        .send_json(ureq::json!({
            "base_model_name": "bluePencilXL_v100.safetensors",
            "negative_prompt": "(embedding:unaestheticXLv31:0.8), low quality, watermark,,verybadimagenegative_v1.3,ng_deepnegative_v1_75t,EasyNegative,badhandv4,rev2-badprompt,easynegative",
            "style_selections": [
                "SAI Anime",
                "SAI Enhance",
                "Fooocus Enhance",
            ],
            "advanced_params": {},
            "require_base64": true,
            "async_process": false,
            "performance_selection": "Extreme Speed",
            "image_prompts": [{
                "cn_img": encoded,
                "cn_stop": 0.5,
                "cn_weight": 0.6,
                "cn_type": "ImagePrompt"
            }]
        }))?;

    #[derive(Deserialize)]
    struct Message {
        base64: String,
    }

    let json: Vec<Message> = resp.into_json()?;
    let img_bytes = general_purpose::STANDARD.decode(&json.get(0).ok_or_else(|| anyhow!("message is empty"))?.base64)?;
    let img = image::load(std::io::Cursor::new(img_bytes), ImageFormat::Png)?;

    Ok(img)
}

// init -> modify1 -> modify2 -> init
#[derive(Debug)]
enum FileChangeMS {
    Init,
    Modify,
}

fn process(
    watch_img: &Path,
    client: &Agent,
    api: &str,
) -> Result<()> {
    let window = create_window("image", Default::default())?;
    let mut ms = FileChangeMS::Init;

    let (tx, rx) = std::sync::mpsc::channel();

    let mut watcher = notify::recommended_watcher(tx)?;
    watcher.watch(watch_img, RecursiveMode::NonRecursive)?;

    loop {
        let e = rx.recv()??;

        println!("event: {:?}, ms: {:?}", e.kind, ms);

        match e.kind {
            EventKind::Modify(_) => {
                match ms {
                    FileChangeMS::Init => ms = FileChangeMS::Modify,
                    FileChangeMS::Modify => {
                        ms = FileChangeMS::Init;
                        println!("image prompt");
                        std::thread::sleep(Duration::from_millis(100));

                        let img = image_prompt(watch_img, client, api)?;
                        window.set_image("image-001", img)?;
                    }
                }
            }
            _ => ()
        }
    }
}

#[derive(Parser)]
#[command(version)]
struct Args {
    fooocus_api: String,
    image: PathBuf,
}

fn main() {
    let args: Args = Args::parse();

    show_image::run_context(|| {
        let img = args.image;
        let client = Agent::new();
        let api = args.fooocus_api;
        process(&img, &client, &api).unwrap();
    });
}