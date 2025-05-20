use clap::Parser;
use futures::{StreamExt, stream::FuturesUnordered};
use reqwest::Client;
use serde_json::{Map, Value, json};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::sync::Semaphore;
use walkdir::WalkDir;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = "locales/en")]
    source: String,

    #[arg(short, long, default_value = "locales")]
    output: String,

    #[arg(short, long, default_value = "de,id,ja")]
    langs: String,

    #[arg(short, long, default_value_t = 10)]
    concurrency: usize,

    #[arg(short = 'u', long, default_value = "http://localhost:5000/translate")]
    url: String,

    #[arg(long, default_value = "")]
    token: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let langs: Vec<&str> = args.langs.split(',').collect();
    let semaphore = Arc::new(Semaphore::new(args.concurrency));
    let client = Client::new();
    let files = find_json_files(&args.source);

    let tasks = files.into_iter().map(|file| {
        let client = client.clone();
        let semaphore = semaphore.clone();
        let langs = langs.clone();
        let source_dir = args.source.clone();
        let output_dir = args.output.clone();
        let url = args.url.clone();
        async move {
            println!("Translating {:?}", file);
            if let Err(e) = translate_file(
                &client,
                semaphore,
                &langs,
                &source_dir,
                &output_dir,
                &url,
                &file,
                &token,
            )
            .await
            {
                eprintln!("❌ Error translating {:?}: {}", file, e);
            }
        }
    });

    futures::future::join_all(tasks).await;
    println!("✅ All translations saved to '{}'", args.output);
    Ok(())
}

fn find_json_files(dir: &str) -> Vec<PathBuf> {
    WalkDir::new(dir)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| {
            e.path().is_file() && e.path().extension().and_then(|s| s.to_str()) == Some("json")
        })
        .map(|e| e.into_path())
        .collect()
}

async fn translate_file(
    client: &Client,
    semaphore: Arc<Semaphore>,
    target_langs: &[&str],
    source_dir: &str,
    output_dir: &str,
    url: &str,
    file_path: &Path,
    token: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let file_content = fs::read_to_string(file_path)?;
    let json: Value = serde_json::from_str(&file_content)?;
    let mut flat_map = BTreeMap::new();
    flatten_json(&json, "".to_string(), &mut flat_map);

    let unique_texts: HashSet<String> = flat_map.values().cloned().collect();
    let mut translations: HashMap<&str, HashMap<String, String>> = HashMap::new();

    for &lang in target_langs {
        let mut text_map = HashMap::new();
        let mut futures = FuturesUnordered::new();

        for text in unique_texts.iter() {
            let client = client.clone();
            let text = text.clone();
            let lang = lang.to_string();
            let url = url.to_string();
            let sem = semaphore.clone();
            futures.push(async move {
                let _permit = sem.acquire_owned().await.unwrap();
                let translated = fetch_translation(&client, &url, &text, &lang, &token)
                    .await
                    .unwrap_or_else(|_| text.clone());
                (text, translated)
            });
        }

        while let Some((orig, trans)) = futures.next().await {
            text_map.insert(orig, trans);
        }

        translations.insert(lang, text_map);
    }

    for &lang in target_langs {
        let mut flat_translated = BTreeMap::new();
        for (k, v) in &flat_map {
            let translated = translations[lang].get(v).unwrap_or(v);
            flat_translated.insert(k.clone(), translated.clone());
        }

        let reconstructed = unflatten_json(&flat_translated);
        let relative = file_path.strip_prefix(source_dir)?;
        let out_path = Path::new(output_dir).join(lang).join(relative);

        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let new_content = serde_json::to_string_pretty(&reconstructed)?;
        if fs::read_to_string(&out_path).unwrap_or_default() != new_content {
            fs::write(out_path, new_content)?;
        }
    }

    Ok(())
}

async fn fetch_translation(
    client: &Client,
    url: &str,
    text: &str,
    target_lang: &str,
    token: &str,
) -> Result<String, reqwest::Error> {
    let payload = json!({
        "q": text,
        "source": "en",
        "target": target_lang,
        "format": "text"
    });

    let mut request = client.post(url).json(&payload);
    if !token.is_empty() {
        request = request.header("Authorization", format!("Bearer {}", token));
    }

    let res = request.send().await?;
    let body: Value = res.json().await?;
    Ok(body["translatedText"].as_str().unwrap_or(text).to_string())
}

fn flatten_json(value: &Value, prefix: String, map: &mut BTreeMap<String, String>) {
    match value {
        Value::Object(obj) => {
            for (k, v) in obj {
                let new_prefix = if prefix.is_empty() {
                    k.clone()
                } else {
                    format!("{}.{}", prefix, k)
                };
                flatten_json(v, new_prefix, map);
            }
        }
        Value::String(s) => {
            map.insert(prefix, s.clone());
        }
        _ => {}
    }
}

fn unflatten_json(flat: &BTreeMap<String, String>) -> Value {
    let mut root = Map::new();
    for (key, val) in flat {
        let parts: Vec<&str> = key.split('.').collect();
        let mut current = &mut root;
        for i in 0..parts.len() {
            if i == parts.len() - 1 {
                current.insert(parts[i].to_string(), Value::String(val.clone()));
            } else {
                current = current
                    .entry(parts[i])
                    .or_insert_with(|| Value::Object(Map::new()))
                    .as_object_mut()
                    .unwrap();
            }
        }
    }
    Value::Object(root)
}
