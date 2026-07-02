use serde_json::{json, Value};

pub struct I18n {
    locale: String,
    en: Value,
    es: Value,
}

impl I18n {
    pub fn new(locale: &str) -> Self {
        let en = json!({
            "app": {
                "name": "Nimbo",
                "loading": "Loading...",
                "error": "Error"
            },
            "weather": {
                "title": "Weather",
                "feels_like": "Feels like",
                "humidity": "Humidity",
                "wind": "Wind",
                "loading": "Loading weather...",
                "error": "Could not load weather"
            },
            "media": {
                "title": "Now Playing",
                "no_media": "No media playing",
                "play": "Play",
                "pause": "Pause",
                "next": "Next",
                "previous": "Previous"
            },
            "stats": {
                "title": "System Stats",
                "cpu": "CPU",
                "memory": "Memory",
                "disk": "Disk",
                "network": "Network"
            },
            "tray": {
                "show_hide": "Show / Hide",
                "quit": "Quit"
            },
            "settings": {
                "title": "Settings",
                "language": "Language",
                "theme": "Theme",
                "theme_light": "Light",
                "theme_dark": "Dark",
                "theme_glass": "Glass"
            }
        });

        let es = json!({
            "app": {
                "name": "Nimbo",
                "loading": "Cargando...",
                "error": "Error"
            },
            "weather": {
                "title": "Clima",
                "feels_like": "Sensación térmica",
                "humidity": "Humedad",
                "wind": "Viento",
                "loading": "Cargando clima...",
                "error": "No se pudo cargar el clima"
            },
            "media": {
                "title": "Reproduciendo",
                "no_media": "Sin medios en reproducción",
                "play": "Reproducir",
                "pause": "Pausar",
                "next": "Siguiente",
                "previous": "Anterior"
            },
            "stats": {
                "title": "Estadísticas del sistema",
                "cpu": "CPU",
                "memory": "Memoria",
                "disk": "Disco",
                "network": "Red"
            },
            "tray": {
                "show_hide": "Mostrar / Ocultar",
                "quit": "Salir"
            },
            "settings": {
                "title": "Configuración",
                "language": "Idioma",
                "theme": "Tema",
                "theme_light": "Claro",
                "theme_dark": "Oscuro",
                "theme_glass": "Cristal"
            }
        });

        Self {
            locale: locale.to_string(),
            en,
            es,
        }
    }

    /// Look up a dot-separated key (e.g. "weather.title") in the active locale dict.
    /// Falls back to English, then returns the key itself if not found.
    pub fn t(&self, key: &str) -> String {
        let dict = match self.locale.as_str() {
            "es" => &self.es,
            _    => &self.en,
        };

        let value = Self::resolve(dict, key)
            .or_else(|| Self::resolve(&self.en, key));

        value
            .and_then(|v| v.as_str().map(str::to_string))
            .unwrap_or_else(|| key.to_string())
    }

    /// Walk a dot-separated key path through a JSON Value.
    fn resolve<'a>(dict: &'a Value, key: &str) -> Option<&'a Value> {
        let mut current = dict;
        for segment in key.split('.') {
            current = current.get(segment)?;
        }
        Some(current)
    }

    pub fn set_locale(&mut self, locale: &str) {
        self.locale = locale.to_string();
    }

    pub fn locale(&self) -> &str {
        &self.locale
    }
}

impl Default for I18n {
    fn default() -> Self {
        Self::new("en")
    }
}
