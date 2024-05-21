use serde_json::{json, Value};
use std::collections::HashMap;
use std::error::Error;
use std::io::Cursor;
use image::{GenericImageView, ImageOutputFormat};
use base64::{Engine as _, engine::general_purpose};
use std::time::Duration;

#[no_mangle]
pub fn recognize(
    base64: &str,                   // 图像Base64
    lang: &str,                     // 识别语言
    needs: HashMap<String, String>, // 插件需要的其他参数,由info.json定义
) -> Result<Value, Box<dyn Error>> {
    let timeout = match needs.get("timeout") {
        Some(timeout) => timeout.parse::<u64>().unwrap_or(30),
        None => 30, // 默认超时时间为30秒
    };

    let client = reqwest::blocking::ClientBuilder::new()
        .timeout(Duration::from_secs(timeout))
        .build()?;

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

    // 如果图片大小小于1MB，则直接使用原始图片
    let compressed_base64 = if img_data.len() <= 1024 * 1024 {
        base64.to_string()
    } else {
        // 读取图片
        let img = image::load_from_memory(&img_data)?;

        // 压缩图片
        let max_size = 1400;
        let (width, height) = img.dimensions();
        let ratio = max_size as f32 / width.max(height) as f32;
        let new_width = (width as f32 * ratio) as u32;
        let new_height = (height as f32 * ratio) as u32;
        let compressed_img = img.resize(new_width, new_height, image::imageops::FilterType::Triangle);

        // 将压缩后的图片转换为 base64，并确保大小不超过 1MB
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
        general_purpose::STANDARD.encode(&buf)
    };

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

    let response = client
        .post(&endpoint)
        .header("Accept", "application/json, text/event-stream")
        .header("Content-Type", "application/json")
        .header("authorization", format!("Bearer {}", apikey))
        .json(&request_body)
        .send()?;

    let response_text = response.text()?;
    let response_json: Value = serde_json::from_str(&response_text).map_err(|e| {
        eprintln!("Error decoding response body: {}", e);
        eprintln!("Response body: {}", response_text);
        "Response Parse Error"
    })?;

    match response_json["choices"][0]["message"]["content"].as_str() {
        Some(result) => Ok(Value::String(result.to_string())),
        None => Err("Response Parse Error".into()),
    }
}
