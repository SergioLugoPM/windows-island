import { useId } from "react";

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

/** Layered, gradient-shaded animated SVG icon matching the current weather condition. */
export function WeatherIcon({ iconCode, size = 42 }: Props) {
  const category = getCategory(iconCode);
  const uid = useId().replace(/:/g, "");

  return (
    <svg width={size} height={size} viewBox="0 0 64 64" className={`weather-icon-svg weather-icon-${category}`}>
      <defs>
        <radialGradient id={`${uid}-sunGlow`} cx="50%" cy="50%" r="60%">
          <stop offset="0%" stopColor="rgba(255,230,150,0.35)" />
          <stop offset="100%" stopColor="rgba(255,230,150,0)" />
        </radialGradient>
        <linearGradient id={`${uid}-sunCore`} x1="0%" y1="0%" x2="100%" y2="100%">
          <stop offset="0%" stopColor="#FFE9A8" />
          <stop offset="100%" stopColor="#FFB648" />
        </linearGradient>
        <linearGradient id={`${uid}-cloudBody`} x1="0%" y1="0%" x2="0%" y2="100%">
          <stop offset="0%" stopColor="#F4F7FC" />
          <stop offset="100%" stopColor="#C7D2E3" />
        </linearGradient>
        <linearGradient id={`${uid}-cloudBack`} x1="0%" y1="0%" x2="0%" y2="100%">
          <stop offset="0%" stopColor="#B9C6DC" />
          <stop offset="100%" stopColor="#94A4C2" />
        </linearGradient>
        <linearGradient id={`${uid}-mistCloud`} x1="0%" y1="0%" x2="0%" y2="100%">
          <stop offset="0%" stopColor="#E4EAF3" />
          <stop offset="100%" stopColor="#AFBBD1" />
        </linearGradient>
        <linearGradient id={`${uid}-mistLine`} x1="0%" y1="0%" x2="100%" y2="0%">
          <stop offset="0%" stopColor="rgba(180,195,220,0)" />
          <stop offset="50%" stopColor="rgba(180,195,220,0.95)" />
          <stop offset="100%" stopColor="rgba(180,195,220,0)" />
        </linearGradient>
        <linearGradient id={`${uid}-rainCloud`} x1="0%" y1="0%" x2="0%" y2="100%">
          <stop offset="0%" stopColor="#DCE3EE" />
          <stop offset="100%" stopColor="#8A9AB8" />
        </linearGradient>
        <linearGradient id={`${uid}-dropG`} x1="0%" y1="0%" x2="0%" y2="100%">
          <stop offset="0%" stopColor="rgba(90,160,255,0)" />
          <stop offset="100%" stopColor="#5AA0FF" />
        </linearGradient>
        <linearGradient id={`${uid}-snowCloud`} x1="0%" y1="0%" x2="0%" y2="100%">
          <stop offset="0%" stopColor="#EAF1FB" />
          <stop offset="100%" stopColor="#B7C6DE" />
        </linearGradient>
        <linearGradient id={`${uid}-stormCloud`} x1="0%" y1="0%" x2="0%" y2="100%">
          <stop offset="0%" stopColor="#B9C2D6" />
          <stop offset="100%" stopColor="#5D6B8C" />
        </linearGradient>
        <linearGradient id={`${uid}-boltG`} x1="0%" y1="0%" x2="0%" y2="100%">
          <stop offset="0%" stopColor="#FFE97A" />
          <stop offset="100%" stopColor="#FFB020" />
        </linearGradient>
      </defs>

      {category === "clear" && (
        <>
          <circle cx="32" cy="32" r="30" fill={`url(#${uid}-sunGlow)`} className="wi-sun-core" />
          <g className="wi-sun-rays" stroke="#FFCE7A" strokeWidth="2.5" strokeLinecap="round" opacity="0.85">
            <line x1="32" y1="6" x2="32" y2="13" />
            <line x1="32" y1="51" x2="32" y2="58" />
            <line x1="6" y1="32" x2="13" y2="32" />
            <line x1="51" y1="32" x2="58" y2="32" />
            <line x1="13.8" y1="13.8" x2="18.6" y2="18.6" />
            <line x1="45.4" y1="45.4" x2="50.2" y2="50.2" />
            <line x1="50.2" y1="13.8" x2="45.4" y2="18.6" />
            <line x1="18.6" y1="45.4" x2="13.8" y2="50.2" />
          </g>
          <circle cx="32" cy="32" r="14" fill={`url(#${uid}-sunCore)`} />
          <ellipse cx="27" cy="26" rx="5" ry="3" fill="rgba(255,255,255,0.55)" />
        </>
      )}

      {category === "clouds" && (
        <g className="wi-cloud-bob">
          <ellipse cx="24" cy="30" rx="13" ry="11" fill={`url(#${uid}-cloudBack)`} opacity="0.8" />
          <path fill={`url(#${uid}-cloudBody)`} d="M18 46a12 12 0 0 1-2-23.8A15.5 15.5 0 0 1 45 20a10.5 10.5 0 0 1 3 20.6V46Z" />
          <ellipse cx="26" cy="27" rx="6" ry="3" fill="rgba(255,255,255,0.65)" />
        </g>
      )}

      {category === "mist" && (
        <>
          <path fill={`url(#${uid}-mistCloud)`} opacity="0.85" d="M18 28a10 10 0 0 1-1-19.5A13 13 0 0 1 40 10a9 9 0 0 1 2 17.3V28Z" />
          <g className="wi-mist-lines">
            <rect x="6" y="38" width="52" height="4" rx="2" fill={`url(#${uid}-mistLine)`} />
            <rect x="12" y="46" width="40" height="4" rx="2" fill={`url(#${uid}-mistLine)`} />
            <rect x="4" y="54" width="56" height="4" rx="2" fill={`url(#${uid}-mistLine)`} />
          </g>
        </>
      )}

      {(category === "rain" || category === "storm") && (
        <>
          <path fill={category === "storm" ? `url(#${uid}-stormCloud)` : `url(#${uid}-rainCloud)`}
            d="M18 34a11 11 0 0 1-1-21.5A14.3 14.3 0 0 1 42 14a9.6 9.6 0 0 1 2 19V34Z" />
          <ellipse cx="26" cy="16" rx="5" ry="2.6" fill="rgba(255,255,255,0.5)" />
          <g className="wi-rain-drops" stroke={`url(#${uid}-dropG)`} strokeWidth="3.4" strokeLinecap="round">
            <line x1="21" y1="42" x2="17" y2="55" />
            <line x1="33" y1="42" x2="29" y2="55" />
            <line x1="45" y1="42" x2="41" y2="55" />
          </g>
          {category === "storm" && (
            <path className="wi-bolt" fill={`url(#${uid}-boltG)`} d="M34 38 L23 54 L30 54 L27 62 L41 44 L33 44 Z" />
          )}
        </>
      )}

      {category === "snow" && (
        <>
          <path fill={`url(#${uid}-snowCloud)`} d="M18 34a11 11 0 0 1-1-21.5A14.3 14.3 0 0 1 42 14a9.6 9.6 0 0 1 2 19V34Z" />
          <ellipse cx="26" cy="16" rx="5" ry="2.6" fill="rgba(255,255,255,0.55)" />
          <g className="wi-snow-flakes" fill="none" stroke="#CFE0FF" strokeWidth="2" strokeLinecap="round">
            <g className="wi-flake" transform="translate(20,47)">
              <line x1="-5" y1="0" x2="5" y2="0" /><line x1="0" y1="-5" x2="0" y2="5" />
              <line x1="-3.5" y1="-3.5" x2="3.5" y2="3.5" /><line x1="-3.5" y1="3.5" x2="3.5" y2="-3.5" />
            </g>
            <g className="wi-flake" transform="translate(44,47) scale(0.8)">
              <line x1="-5" y1="0" x2="5" y2="0" /><line x1="0" y1="-5" x2="0" y2="5" />
              <line x1="-3.5" y1="-3.5" x2="3.5" y2="3.5" /><line x1="-3.5" y1="3.5" x2="3.5" y2="-3.5" />
            </g>
            <g className="wi-flake" transform="translate(32,57) scale(0.65)">
              <line x1="-5" y1="0" x2="5" y2="0" /><line x1="0" y1="-5" x2="0" y2="5" />
              <line x1="-3.5" y1="-3.5" x2="3.5" y2="3.5" /><line x1="-3.5" y1="3.5" x2="3.5" y2="-3.5" />
            </g>
          </g>
        </>
      )}
    </svg>
  );
}
