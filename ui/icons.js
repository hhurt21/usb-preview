// 機器の絵 (SVG)。色は currentColor (線) と --ic-fill (面) で、明るい画面・暗い画面の両方に合わせる。
// どれも 64×64 の中に描いている。
const ICON_SVG = {
  pc: `<rect x="34" y="8" width="20" height="48" rx="3"/><circle cx="44" cy="16" r="2.5" class="f"/><line x1="38" y1="24" x2="50" y2="24"/><line x1="38" y1="29" x2="50" y2="29"/>
       <rect x="6" y="14" width="24" height="18" rx="2"/><rect x="9" y="17" width="18" height="12" class="f"/><line x1="18" y1="32" x2="18" y2="40"/><line x1="11" y1="41" x2="25" y2="41"/>
       <rect x="6" y="46" width="24" height="8" rx="2"/>`,
  monitor: `<rect x="6" y="10" width="52" height="34" rx="3"/><rect x="10" y="14" width="44" height="26" class="f"/><path d="M26 44 L24 54 H40 L38 44"/><line x1="20" y1="54" x2="44" y2="54"/>`,
  gpu: `<rect x="4" y="16" width="52" height="30" rx="3"/><circle cx="20" cy="31" r="9"/><circle cx="20" cy="31" r="3" class="f"/><circle cx="41" cy="31" r="9"/><circle cx="41" cy="31" r="3" class="f"/>
        <path d="M10 46 V52 H34 V46"/><line x1="56" y1="22" x2="60" y2="22"/><line x1="56" y1="40" x2="60" y2="40"/>`,
  cpu: `<rect x="16" y="16" width="32" height="32" rx="3"/><rect x="24" y="24" width="16" height="16" class="f"/>
        <path d="M24 16 V8 M32 16 V8 M40 16 V8 M24 48 V56 M32 48 V56 M40 48 V56 M16 24 H8 M16 32 H8 M16 40 H8 M48 24 H56 M48 32 H56 M48 40 H56"/>`,
  usb: `<rect x="22" y="6" width="20" height="16" rx="2"/><rect x="26" y="10" width="4" height="4" class="f"/><rect x="34" y="10" width="4" height="4" class="f"/>
        <rect x="18" y="22" width="28" height="26" rx="4" class="f"/><path d="M32 48 V58"/>`,
  hub: `<rect x="6" y="24" width="52" height="18" rx="4"/><rect x="12" y="30" width="8" height="5" class="f"/><rect x="24" y="30" width="8" height="5" class="f"/><rect x="36" y="30" width="8" height="5" class="f"/><rect x="48" y="30" width="5" height="5" class="f"/>
        <path d="M32 24 V12 H40"/>`,
  keyboard: `<rect x="4" y="18" width="56" height="30" rx="4"/>
        <g class="f"><rect x="9" y="23" width="6" height="5" rx="1"/><rect x="18" y="23" width="6" height="5" rx="1"/><rect x="27" y="23" width="6" height="5" rx="1"/><rect x="36" y="23" width="6" height="5" rx="1"/><rect x="45" y="23" width="10" height="5" rx="1"/>
        <rect x="9" y="31" width="9" height="5" rx="1"/><rect x="21" y="31" width="6" height="5" rx="1"/><rect x="30" y="31" width="6" height="5" rx="1"/><rect x="39" y="31" width="6" height="5" rx="1"/><rect x="48" y="31" width="7" height="5" rx="1"/>
        <rect x="18" y="39" width="28" height="5" rx="1"/></g>`,
  mouse: `<rect x="18" y="8" width="28" height="48" rx="14"/><line x1="32" y1="8" x2="32" y2="26"/><line x1="18" y1="26" x2="46" y2="26"/><rect x="30" y="13" width="4" height="8" rx="2" class="f"/>`,
  gamepad: `<path d="M16 20 H48 C56 20 60 30 60 40 C60 50 54 54 49 50 L42 42 H22 L15 50 C10 54 4 50 4 40 C4 30 8 20 16 20 Z"/>
        <path d="M16 28 V38 M11 33 H21"/><circle cx="44" cy="29" r="2.5" class="f"/><circle cx="50" cy="35" r="2.5" class="f"/><circle cx="38" cy="35" r="2.5" class="f"/><circle cx="44" cy="41" r="2.5" class="f"/>`,
  headphones: `<path d="M10 40 V32 C10 19 20 10 32 10 C44 10 54 19 54 32 V40"/><rect x="6" y="36" width="12" height="18" rx="4" class="f"/><rect x="46" y="36" width="12" height="18" rx="4" class="f"/>`,
  earbuds: `<circle cx="20" cy="24" r="10"/><circle cx="20" cy="24" r="4" class="f"/><path d="M24 33 L28 54"/><circle cx="44" cy="24" r="10"/><circle cx="44" cy="24" r="4" class="f"/><path d="M40 33 L36 54"/>`,
  speaker: `<rect x="14" y="6" width="36" height="52" rx="4"/><circle cx="32" cy="18" r="5" class="f"/><circle cx="32" cy="40" r="11"/><circle cx="32" cy="40" r="4" class="f"/>`,
  mic: `<rect x="23" y="6" width="18" height="30" rx="9" class="f"/><path d="M15 30 C15 41 22 47 32 47 C42 47 49 41 49 30"/><line x1="32" y1="47" x2="32" y2="56"/><line x1="22" y1="57" x2="42" y2="57"/>`,
  audioif: `<rect x="4" y="18" width="56" height="28" rx="4"/><circle cx="16" cy="32" r="6"/><circle cx="16" cy="32" r="2" class="f"/><circle cx="32" cy="32" r="6"/><circle cx="32" cy="32" r="2" class="f"/>
        <circle cx="48" cy="28" r="3" class="f"/><line x1="44" y1="38" x2="53" y2="38"/>`,
  bluetooth: `<path d="M22 20 L42 40 L32 50 V14 L42 24 L22 44"/>`,
  ethernet: `<rect x="12" y="16" width="40" height="30" rx="3"/><path d="M20 46 V54 H44 V46" /><g class="f"><rect x="19" y="22" width="4" height="9"/><rect x="26" y="22" width="4" height="9"/><rect x="34" y="22" width="4" height="9"/><rect x="41" y="22" width="4" height="9"/></g>`,
  wifi: `<path d="M6 26 C20 12 44 12 58 26"/><path d="M14 34 C24 25 40 25 50 34"/><path d="M22 42 C28 37 36 37 42 42"/><circle cx="32" cy="50" r="4" class="f"/>`,
  hdd: `<rect x="10" y="6" width="44" height="52" rx="4"/><circle cx="32" cy="27" r="14"/><circle cx="32" cy="27" r="3" class="f"/><path d="M32 27 L46 48"/><circle cx="17" cy="51" r="2" class="f"/>`,
  ssd: `<rect x="6" y="20" width="52" height="24" rx="3"/><g class="f"><rect x="12" y="25" width="10" height="14" rx="1"/><rect x="26" y="25" width="10" height="14" rx="1"/></g><path d="M44 25 V39 M48 25 V39 M52 25 V39"/>`,
  usbdrive: `<rect x="20" y="22" width="24" height="36" rx="4" class="f"/><rect x="24" y="6" width="16" height="16" rx="1"/><rect x="27" y="10" width="3" height="4"/><rect x="34" y="10" width="3" height="4"/>`,
  camera: `<rect x="6" y="18" width="52" height="34" rx="4"/><path d="M22 18 L26 10 H38 L42 18"/><circle cx="32" cy="35" r="10"/><circle cx="32" cy="35" r="4" class="f"/>`,
  printer: `<rect x="16" y="6" width="32" height="16"/><rect x="6" y="22" width="52" height="22" rx="3"/><rect x="16" y="36" width="32" height="20" class="f"/><circle cx="50" cy="30" r="2" class="f"/>`,
  serial: `<path d="M8 22 H56 L50 44 H14 Z"/><g class="f"><circle cx="20" cy="30" r="2"/><circle cx="28" cy="30" r="2"/><circle cx="36" cy="30" r="2"/><circle cx="44" cy="30" r="2"/><circle cx="24" cy="37" r="2"/><circle cx="32" cy="37" r="2"/><circle cx="40" cy="37" r="2"/></g>`,
  chip: `<rect x="14" y="14" width="36" height="36" rx="4"/><path d="M22 14 V6 M32 14 V6 M42 14 V6 M22 50 V58 M32 50 V58 M42 50 V58 M14 22 H6 M14 32 H6 M14 42 H6 M50 22 H58 M50 32 H58 M50 42 H58"/><circle cx="24" cy="24" r="3" class="f"/>`,
  memory: `<rect x="4" y="20" width="56" height="20" rx="2"/><g class="f"><rect x="9" y="24" width="8" height="10"/><rect x="21" y="24" width="8" height="10"/><rect x="35" y="24" width="8" height="10"/><rect x="47" y="24" width="8" height="10"/></g><path d="M8 40 V46 M14 40 V46 M20 40 V46 M26 40 V46 M38 40 V46 M44 40 V46 M50 40 V46 M56 40 V46"/>`,
  sound: `<path d="M8 26 H18 L30 14 V50 L18 38 H8 Z" class="f"/><path d="M38 24 C42 28 42 36 38 40"/><path d="M44 18 C52 26 52 38 44 46"/>`,
  input: `<rect x="4" y="30" width="38" height="22" rx="3"/><g class="f"><rect x="9" y="35" width="5" height="4"/><rect x="17" y="35" width="5" height="4"/><rect x="25" y="35" width="5" height="4"/><rect x="33" y="35" width="5" height="4"/><rect x="12" y="43" width="22" height="4"/></g>
          <rect x="44" y="10" width="16" height="28" rx="8"/><line x1="52" y1="10" x2="52" y2="20"/>`,
  other: `<circle cx="16" cy="32" r="5" class="f"/><circle cx="32" cy="32" r="5" class="f"/><circle cx="48" cy="32" r="5" class="f"/>`,
};

/** 絵の SVG 文字列。size は表示の大きさ (px)。 */
function iconSvg(name, size = 28) {
  const body = ICON_SVG[name] || ICON_SVG.other;
  return `<svg class="ic" width="${size}" height="${size}" viewBox="0 0 64 64" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round">${body}</svg>`;
}
