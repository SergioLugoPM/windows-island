import { useState, useEffect } from "react";
import { WeatherBackdrop } from "./WeatherBackdrop";

export interface WeatherInfo {
  temp_c: number;
  description: string;
  icon_code: string;
  city: string;
  humidity: number;
  feels_like_c: number;
  wind_kmph: number;
}

const WEATHER_ICONS: Record<string, string> = {
  "113": "☀️", "116": "⛅", "119": "☁️", "122": "☁️",
  "143": "🌫️", "176": "🌦️", "179": "🌨️", "182": "🌧️",
  "185": "🌧️", "200": "⛈️", "227": "❄️", "230": "❄️",
  "248": "🌫️", "260": "🌫️", "263": "🌦️", "266": "🌦️",
  "281": "🌧️", "284": "🌧️", "293": "🌧️", "296": "🌧️",
  "299": "🌧️", "302": "🌧️", "305": "🌧️", "308": "🌧️",
  "311": "🌨️", "314": "🌨️", "317": "🌨️", "320": "🌨️",
  "353": "🌦️", "356": "🌧️", "359": "🌧️", "386": "⛈️",
  "389": "⛈️", "392": "⛈️", "395": "❄️",
};

function getIcon(code: string): string {
  return WEATHER_ICONS[code] ?? "🌡️";
}

async function fetchWeather(city: string): Promise<WeatherInfo | null> {
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    return await invoke<WeatherInfo>("get_weather", { city });
  } catch {
    return null;
  }
}

interface Props {
  city?: string;
  compact?: boolean;
}

export function WeatherView({ city = "auto", compact = false }: Props) {
  const [weather, setWeather] = useState<WeatherInfo | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    setLoading(true);
    fetchWeather(city).then((w) => {
      setWeather(w);
      setLoading(false);
    });
    // Refresh every 30 min
    const id = setInterval(() => fetchWeather(city).then(setWeather), 30 * 60 * 1000);
    return () => clearInterval(id);
  }, [city]);

  if (loading) {
    return (
      <div className="weather-row" style={{ justifyContent: "center" }}>
        <span className="empty-label">Cargando clima…</span>
      </div>
    );
  }

  if (!weather) {
    return (
      <div className="weather-row" style={{ justifyContent: "center" }}>
        <span className="empty-label">Sin datos de clima</span>
      </div>
    );
  }

  if (compact) {
    return (
      <div className="weather-row">
        <span style={{ fontSize: 48, lineHeight: 1, flexShrink: 0 }}>{getIcon(weather.icon_code)}</span>
        <span className="weather-temp" style={{ fontSize: 28 }}>
          {weather.temp_c}°
        </span>
      </div>
    );
  }

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 10, width: "100%", position: "relative" }}>
      <WeatherBackdrop iconCode={weather.icon_code} />
      <div className="weather-row" style={{ position: "relative", zIndex: 1 }}>
        <div className="weather-icon">{getIcon(weather.icon_code)}</div>
        <div className="weather-info">
          <div className="weather-temp">{weather.temp_c}°C</div>
          <div className="weather-desc">{weather.description}</div>
          <div className="weather-city">{weather.city}</div>
        </div>
      </div>
      <div className="stat-card-grid" style={{ position: "relative", zIndex: 1 }}>
        <div className="stat-card">
          <div className="stat-card-header">Humidity</div>
          <div style={{ fontSize: 14, fontWeight: 700, color: "rgba(230,235,255,0.95)" }}>
            {weather.humidity}%
          </div>
        </div>
        <div className="stat-card">
          <div className="stat-card-header">Feels like</div>
          <div style={{ fontSize: 14, fontWeight: 700, color: "rgba(230,235,255,0.95)" }}>
            {weather.feels_like_c}°C
          </div>
        </div>
        <div className="stat-card">
          <div className="stat-card-header">Wind</div>
          <div style={{ fontSize: 14, fontWeight: 700, color: "rgba(230,235,255,0.95)" }}>
            {weather.wind_kmph} km/h
          </div>
        </div>
      </div>
    </div>
  );
}
