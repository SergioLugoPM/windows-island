interface Props {
  iconCode: string;
  size?: number;
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

/** Small hand-drawn animated SVG icon matching the current weather condition. */
export function WeatherIcon({ iconCode, size = 42 }: Props) {
  const category = getCategory(iconCode);

  return (
    <svg width={size} height={size} viewBox="0 0 64 64" className={`weather-icon-svg weather-icon-${category}`}>
      {category === "clear" && (
        <g className="wi-sun">
          <circle cx="32" cy="32" r="14" fill="currentColor" className="wi-sun-core" />
          <g className="wi-sun-rays" stroke="currentColor" strokeWidth="3" strokeLinecap="round">
            <line x1="32" y1="4"  x2="32" y2="12" />
            <line x1="32" y1="52" x2="32" y2="60" />
            <line x1="4"  y1="32" x2="12" y2="32" />
            <line x1="52" y1="32" x2="60" y2="32" />
            <line x1="12" y1="12" x2="17" y2="17" />
            <line x1="47" y1="47" x2="52" y2="52" />
            <line x1="52" y1="12" x2="47" y2="17" />
            <line x1="17" y1="47" x2="12" y2="52" />
          </g>
        </g>
      )}

      {category === "clouds" && (
        <g className="wi-cloud-bob">
          <path fill="currentColor" d="M20 42a10 10 0 0 1-2-19.8A13 13 0 0 1 43 18a9 9 0 0 1 3 17.5V42Z" />
        </g>
      )}

      {(category === "mist") && (
        <g>
          <path fill="currentColor" opacity="0.7" d="M20 26a10 10 0 0 1-1-19.5A13 13 0 0 1 42 8a9 9 0 0 1 2 17.3V26Z" />
          <g className="wi-mist-lines" stroke="currentColor" strokeWidth="3" strokeLinecap="round" opacity="0.75">
            <line x1="10" y1="42" x2="54" y2="42" />
            <line x1="16" y1="50" x2="48" y2="50" />
            <line x1="8"  y1="58" x2="56" y2="58" />
          </g>
        </g>
      )}

      {(category === "rain" || category === "storm") && (
        <g>
          <path fill="currentColor" d="M20 34a10 10 0 0 1-1-19.5A13 13 0 0 1 42 16a9 9 0 0 1 2 17.3V34Z" />
          <g className="wi-rain-drops" stroke="currentColor" strokeWidth="3" strokeLinecap="round">
            <line x1="20" y1="42" x2="17" y2="52" />
            <line x1="32" y1="42" x2="29" y2="52" />
            <line x1="44" y1="42" x2="41" y2="52" />
          </g>
          {category === "storm" && (
            <path className="wi-bolt" fill="currentColor" d="M34 40 L24 54 L30 54 L28 62 L40 46 L33 46 Z" />
          )}
        </g>
      )}

      {category === "snow" && (
        <g>
          <path fill="currentColor" d="M20 34a10 10 0 0 1-1-19.5A13 13 0 0 1 42 16a9 9 0 0 1 2 17.3V34Z" />
          <g className="wi-snow-flakes" fill="currentColor">
            <circle cx="20" cy="46" r="2.5" />
            <circle cx="32" cy="50" r="2.5" />
            <circle cx="44" cy="46" r="2.5" />
            <circle cx="26" cy="58" r="2.5" />
            <circle cx="38" cy="58" r="2.5" />
          </g>
        </g>
      )}
    </svg>
  );
}
