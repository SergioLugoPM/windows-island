interface Props {
  iconCode: string;
}

type Category = "clear" | "clouds" | "mist" | "rain" | "snow" | "storm";

const CLEAR = new Set(["113"]);
const CLOUDS = new Set(["116", "119", "122"]);
const MIST = new Set(["143", "248", "260"]);
const SNOW = new Set([
  "179", "182", "227", "230", "311", "314", "317", "320",
  "323", "326", "329", "332", "335", "338", "350", "362",
  "365", "368", "371", "374", "377", "392", "395",
]);
const STORM = new Set(["200", "386", "389"]);

function getCategory(code: string): Category {
  if (CLEAR.has(code)) return "clear";
  if (CLOUDS.has(code)) return "clouds";
  if (MIST.has(code)) return "mist";
  if (STORM.has(code)) return "storm";
  if (SNOW.has(code)) return "snow";
  return "rain";
}

/** Ambient, CSS-only background animation matching the current weather condition. */
export function WeatherBackdrop({ iconCode }: Props) {
  const category = getCategory(iconCode);

  return (
    <div className={`weather-backdrop weather-backdrop-${category}`} aria-hidden="true">
      {category === "clear" && <div className="weather-bd-sun" />}

      {category === "clouds" && (
        <>
          <div className="weather-bd-cloud weather-bd-cloud-1" />
          <div className="weather-bd-cloud weather-bd-cloud-2" />
          <div className="weather-bd-cloud weather-bd-cloud-3" />
        </>
      )}

      {category === "mist" && (
        <>
          <div className="weather-bd-mist weather-bd-mist-1" />
          <div className="weather-bd-mist weather-bd-mist-2" />
        </>
      )}

      {(category === "rain" || category === "storm") && (
        <div className="weather-bd-rain">
          {Array.from({ length: 14 }).map((_, i) => (
            <span key={i} className="weather-bd-drop" style={{
              left: `${(i / 14) * 100 + (i % 3) * 2}%`,
              animationDelay: `${(i % 7) * 0.18}s`,
              animationDuration: `${0.7 + (i % 4) * 0.12}s`,
            }} />
          ))}
        </div>
      )}

      {category === "storm" && <div className="weather-bd-flash" />}

      {category === "snow" && (
        <div className="weather-bd-snow">
          {Array.from({ length: 12 }).map((_, i) => (
            <span key={i} className="weather-bd-flake" style={{
              left: `${(i / 12) * 100 + (i % 4) * 3}%`,
              animationDelay: `${(i % 6) * 0.5}s`,
              animationDuration: `${4 + (i % 5) * 0.8}s`,
            }} />
          ))}
        </div>
      )}
    </div>
  );
}
