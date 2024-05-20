use serde_json::{json, Value};
use std::collections::HashMap;
use std::error::Error;
use std::io::Cursor;
use image::{GenericImageView, ImageOutputFormat};
use base64::{Engine as _, engine::general_purpose};

#[no_mangle]
pub fn recognize(
    base64: &str,                   // 图像Base64
    lang: &str,                     // 识别语言
    needs: HashMap<String, String>, // 插件需要的其他参数,由info.json定义
) -> Result<Value, Box<dyn Error>> {
    let client = reqwest::blocking::ClientBuilder::new().build()?;

    let apikey = match needs.get("apikey") {
        Some(apikey) => apikey.to_string(),
        None => return Err("apikey not found".into()),
    };

    let endpoint = match needs.get("endpoint") {
        Some(endpoint) => endpoint.to_string(),
        None => "https://api.openai.com/v1/chat/completions".to_string(),
    };

    let model = match needs.get("model") {
        Some(model) => model.to_string(),
        None => "gpt-4o".to_string(),
    };

    let prompt = match needs.get("prompt") {
        Some(prompt) => format!("{}\nOutput Language:{}", prompt, lang),
        None => format!("Output Language:{}", lang),
    };

    let stream = match needs.get("stream") {
        Some(stream) => stream.to_lowercase() == "true",
        None => false,
    };

    // 解码 base64 图片数据
    let img_data = general_purpose::STANDARD.decode(base64)?;

    // 读取图片
    let img = image::load_from_memory(&img_data)?;

    // 压缩图片
    let max_size = 1400;
    let (width, height) = img.dimensions();
    let ratio = max_size as f32 / width.max(height) as f32;
    let new_width = (width as f32 * ratio) as u32;
    let new_height = (height as f32 * ratio) as u32;
    let compressed_img = img.resize(new_width, new_height, image::imageops::FilterType::Triangle);

    // 将压缩后的图片转换为 base64,并确保大小不超过 1MB
    let mut quality = 80;
    let mut buf = Vec::new();
    loop {
        buf.clear();
        let mut cursor = Cursor::new(&mut buf);
        compressed_img.write_to(&mut cursor, ImageOutputFormat::Jpeg(quality))?;
        if buf.len() <= 1024 * 1024 {
            break;
        }
        quality -= 5;
        if quality < 10 {
            return Err("Image is too large to compress within 1MB".into());
        }
    }
    let compressed_base64 = general_purpose::STANDARD.encode(&buf);

    let request_body = json!({
        "messages": [
            {
                "role": "system",
                "content": prompt
            },
            {
                "role": "user",
                "content": [
                    {
                        "type": "text",
                        "text": "analyze"
                    },
                    {
                        "type": "image_url",
                        "image_url": {
                            "url": format!("data:image/jpeg;base64,{}", compressed_base64)
                        }
                    }
                ]
            }
        ],
        "stream": stream,
        "model": model,
        "temperature": 0.5,
        "presence_penalty": 0,
        "frequency_penalty": 0,
        "top_p": 1
    });

    let response: Value = client
        .post(&endpoint)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36")
        .header("Accept", "application/json, text/event-stream")
        .header("Content-Type", "application/json")
        .header("authorization", format!("Bearer {}", apikey))
        .json(&request_body)
        .send()?
        .json()?;

    match response["choices"][0]["message"]["content"].as_str() {
        Some(result) => Ok(Value::String(result.to_string())),
        None => Err("Response Parse Error".into()),
    }
}
