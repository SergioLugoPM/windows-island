use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct WeatherInfo {
    pub temp_c: i32,
    pub description: String,
    pub icon_code: String,
    pub city: String,
    pub humidity: i32,
    pub feels_like_c: i32,
    pub wind_kmph: i32,
}

// ─── wttr.in response shapes ──────────────────────────────────────────────────

#[derive(Deserialize)]
struct WttrResponse {
    current_condition: Vec<WttrCurrent>,
    nearest_area: Vec<WttrArea>,
}

#[derive(Deserialize)]
struct WttrCurrent {
    #[serde(rename = "temp_C")]
    temp_c: String,
    #[serde(rename = "FeelsLikeC")]
    feels_like_c: String,
    humidity: String,
    #[serde(rename = "windspeedKmph")]
    windspeed_kmph: String,
    #[serde(rename = "weatherDesc")]
    weather_desc: Vec<WttrValue>,
    #[serde(rename = "weatherCode")]
    weather_code: String,
}

#[derive(Deserialize)]
struct WttrArea {
    #[serde(rename = "areaName")]
    area_name: Vec<WttrValue>,
}

#[derive(Deserialize)]
struct WttrValue {
    value: String,
}

// ─── Fetch ────────────────────────────────────────────────────────────────────

pub async fn get_weather(city: &str) -> Result<WeatherInfo, String> {
    let target = if city.is_empty() || city == "auto" {
        "https://wttr.in/?format=j1".to_string()
    } else {
        let encoded = urlencoding::encode(city);
        format!("https://wttr.in/{encoded}?format=j1")
    };

    let resp = reqwest::Client::builder()
        .user_agent("windows-island/0.1")
        .build()
        .map_err(|e| e.to_string())?
        .get(&target)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json::<WttrResponse>()
        .await
        .map_err(|e| e.to_string())?;

    let cur = resp
        .current_condition
        .into_iter()
        .next()
        .ok_or("No current condition")?;

    let city_name = resp
        .nearest_area
        .into_iter()
        .next()
        .and_then(|a| a.area_name.into_iter().next())
        .map(|v| v.value)
        .unwrap_or_else(|| city.to_string());

    let temp_c: i32 = cur.temp_c.parse().map_err(|e: std::num::ParseIntError| e.to_string())?;

    let humidity: i32 = cur.humidity.parse().unwrap_or(0);
    let feels_like_c: i32 = cur.feels_like_c.parse().unwrap_or(temp_c);
    let wind_kmph: i32 = cur.windspeed_kmph.parse().unwrap_or(0);

    let description = cur
        .weather_desc
        .into_iter()
        .next()
        .map(|v| v.value)
        .unwrap_or_default();

    Ok(WeatherInfo {
        temp_c,
        description,
        icon_code: cur.weather_code,
        city: city_name,
        humidity,
        feels_like_c,
        wind_kmph,
    })
}
