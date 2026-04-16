const pptxgen = require("pptxgenjs");
const pres = new pptxgen();

pres.layout = "LAYOUT_16x9";
pres.author = "SubhamBot";
pres.title = "Why Rust Over C for Embedded Systems";

// ---- palette ----
const BG_DARK  = "1A1B26";
const BG_MID   = "24283B";
const FG       = "C0CAF5";
const FG_DIM   = "7982A9";
const ACCENT   = "F7768E";
const GREEN    = "9ECE6A";
const BLUE     = "7AA2F7";
const YELLOW   = "E0AF68";
const ORANGE   = "FF9E64";

const TITLE_FONT = "Arial Black";
const BODY_FONT  = "Calibri";
const CODE_FONT  = "Consolas";

const cardShadow = () => ({
  type: "outer", color: "000000", blur: 8, offset: 2, angle: 135, opacity: 0.3
});

// =====================================================================
// SLIDE 1 — Title
// =====================================================================
{
  const s = pres.addSlide();
  s.background = { color: BG_DARK };
  s.addShape(pres.shapes.RECTANGLE, { x: 0, y: 0, w: 10, h: 0.06, fill: { color: ACCENT } });

  s.addText("Why Rust Over C", {
    x: 0.8, y: 1.2, w: 8.4, h: 1.2,
    fontSize: 44, fontFace: TITLE_FONT, color: FG, bold: true, margin: 0
  });
  s.addText("for Embedded Systems", {
    x: 0.8, y: 2.2, w: 8.4, h: 0.9,
    fontSize: 36, fontFace: TITLE_FONT, color: ACCENT, bold: true, margin: 0
  });
  s.addText("Real-world lessons from an STM32F429 EEG drowsiness detection system", {
    x: 0.8, y: 3.4, w: 8.4, h: 0.6,
    fontSize: 16, fontFace: BODY_FONT, color: FG_DIM, italic: true, margin: 0
  });
  s.addText("RTIC 1.x  |  no_std  |  no heap  |  Cortex-M4", {
    x: 0.8, y: 4.6, w: 8.4, h: 0.4,
    fontSize: 13, fontFace: CODE_FONT, color: FG_DIM, margin: 0
  });
  s.addShape(pres.shapes.RECTANGLE, { x: 0, y: 5.565, w: 10, h: 0.06, fill: { color: ACCENT } });
}

// =====================================================================
// SLIDE 2 — Memory Safety
// =====================================================================
{
  const s = pres.addSlide();
  s.background = { color: BG_DARK };

  s.addText("Memory Safety at Compile Time", {
    x: 0.6, y: 0.3, w: 8.8, h: 0.7,
    fontSize: 32, fontFace: TITLE_FONT, color: FG, bold: true, margin: 0
  });
  s.addShape(pres.shapes.RECTANGLE, { x: 0.6, y: 0.95, w: 2.2, h: 0.04, fill: { color: ACCENT } });

  // Rust card
  s.addShape(pres.shapes.RECTANGLE, { x: 0.4, y: 1.3, w: 4.4, h: 3.9, fill: { color: BG_MID }, shadow: cardShadow() });
  s.addShape(pres.shapes.RECTANGLE, { x: 0.4, y: 1.3, w: 4.4, h: 0.45, fill: { color: GREEN } });
  s.addText("Rust", { x: 0.6, y: 1.32, w: 4.0, h: 0.42, fontSize: 16, fontFace: TITLE_FONT, color: BG_DARK, bold: true, margin: 0 });
  s.addText([
    { text: "// Lifetime-checked static DMA buffer", options: { color: FG_DIM, fontSize: 10, fontFace: CODE_FONT, breakLine: true } },
    { text: "static mut EEG_DMA_BUF: [u8; 128]", options: { color: GREEN, fontSize: 10, fontFace: CODE_FONT, breakLine: true } },
    { text: "    = [0; 128];", options: { color: GREEN, fontSize: 10, fontFace: CODE_FONT, breakLine: true } },
    { text: "let buf: &'static mut [u8; 128]", options: { color: FG, fontSize: 10, fontFace: CODE_FONT, breakLine: true } },
    { text: "    = unsafe { &mut *addr_of_mut!(..) };", options: { color: FG, fontSize: 10, fontFace: CODE_FONT, breakLine: true } },
    { text: "", options: { fontSize: 6, breakLine: true } },
    { text: "// Beta clamped — no div-by-zero possible", options: { color: FG_DIM, fontSize: 10, fontFace: CODE_FONT, breakLine: true } },
    { text: "let beta = if beta_raw.abs() < 1e-3", options: { color: FG, fontSize: 10, fontFace: CODE_FONT, breakLine: true } },
    { text: "    { 1e-3 } else { beta_raw };", options: { color: FG, fontSize: 10, fontFace: CODE_FONT } },
  ], { x: 0.6, y: 1.9, w: 4.0, h: 3.1, valign: "top" });

  // C card
  s.addShape(pres.shapes.RECTANGLE, { x: 5.2, y: 1.3, w: 4.4, h: 3.9, fill: { color: BG_MID }, shadow: cardShadow() });
  s.addShape(pres.shapes.RECTANGLE, { x: 5.2, y: 1.3, w: 4.4, h: 0.45, fill: { color: ACCENT } });
  s.addText("C Equivalent", { x: 5.4, y: 1.32, w: 4.0, h: 0.42, fontSize: 16, fontFace: TITLE_FONT, color: "FFFFFF", bold: true, margin: 0 });
  s.addText([
    { text: "// Who owns this buffer? Who frees it?", options: { color: FG_DIM, fontSize: 10, fontFace: CODE_FONT, breakLine: true } },
    { text: "uint8_t eeg_buf[128];  // global", options: { color: ACCENT, fontSize: 10, fontFace: CODE_FONT, breakLine: true } },
    { text: "uint8_t* p = eeg_buf;", options: { color: ACCENT, fontSize: 10, fontFace: CODE_FONT, breakLine: true } },
    { text: "// p can alias, dangle, overflow", options: { color: FG_DIM, fontSize: 10, fontFace: CODE_FONT, breakLine: true } },
    { text: "// Nothing prevents p[200] = 0xFF;", options: { color: FG_DIM, fontSize: 10, fontFace: CODE_FONT, breakLine: true } },
    { text: "", options: { fontSize: 6, breakLine: true } },
    { text: "// Division by zero is YOUR problem", options: { color: FG_DIM, fontSize: 10, fontFace: CODE_FONT, breakLine: true } },
    { text: "float ratio = alpha / beta;", options: { color: ACCENT, fontSize: 10, fontFace: CODE_FONT, breakLine: true } },
    { text: "// if beta==0 → undefined behaviour", options: { color: YELLOW, fontSize: 10, fontFace: CODE_FONT } },
  ], { x: 5.4, y: 1.9, w: 4.0, h: 3.1, valign: "top" });

  s.addText("Ownership + lifetimes eliminate use-after-free, double-free, and buffer overflow — at zero runtime cost.", {
    x: 0.6, y: 5.05, w: 8.8, h: 0.45, fontSize: 12, fontFace: BODY_FONT, color: FG_DIM, italic: true, margin: 0
  });
}

// =====================================================================
// SLIDE 3 — Compiler Guarantees
// =====================================================================
{
  const s = pres.addSlide();
  s.background = { color: BG_DARK };

  s.addText("The Compiler as Safety Net", {
    x: 0.6, y: 0.3, w: 8.8, h: 0.7,
    fontSize: 32, fontFace: TITLE_FONT, color: FG, bold: true, margin: 0
  });
  s.addShape(pres.shapes.RECTANGLE, { x: 0.6, y: 0.95, w: 2.2, h: 0.04, fill: { color: BLUE } });

  s.addShape(pres.shapes.RECTANGLE, { x: 0.4, y: 1.3, w: 4.4, h: 3.6, fill: { color: BG_MID }, shadow: cardShadow() });
  s.addShape(pres.shapes.RECTANGLE, { x: 0.4, y: 1.3, w: 4.4, h: 0.45, fill: { color: GREEN } });
  s.addText("Rust — exhaustive match", { x: 0.6, y: 1.32, w: 4.0, h: 0.42, fontSize: 14, fontFace: TITLE_FONT, color: BG_DARK, bold: true, margin: 0 });
  s.addText([
    { text: "match (self.state, touched) {", options: { color: BLUE, fontSize: 10, fontFace: CODE_FONT, breakLine: true } },
    { text: "  (Red, _)         => {}", options: { color: FG, fontSize: 10, fontFace: CODE_FONT, breakLine: true } },
    { text: "  (Green, true)    => {}", options: { color: FG, fontSize: 10, fontFace: CODE_FONT, breakLine: true } },
    { text: "  (Yellow, true)   => go_green(),", options: { color: FG, fontSize: 10, fontFace: CODE_FONT, breakLine: true } },
    { text: "  (Orange, true)   => go_green(),", options: { color: FG, fontSize: 10, fontFace: CODE_FONT, breakLine: true } },
    { text: "  (Green, false)   => go_yellow(),", options: { color: FG, fontSize: 10, fontFace: CODE_FONT, breakLine: true } },
    { text: "  (Yellow, false)  => {}", options: { color: FG, fontSize: 10, fontFace: CODE_FONT, breakLine: true } },
    { text: "  (Orange, false)  => {}", options: { color: FG, fontSize: 10, fontFace: CODE_FONT, breakLine: true } },
    { text: "} // miss a case? WON'T COMPILE", options: { color: GREEN, fontSize: 10, fontFace: CODE_FONT } },
  ], { x: 0.6, y: 1.9, w: 4.0, h: 2.8, valign: "top" });

  s.addShape(pres.shapes.RECTANGLE, { x: 5.2, y: 1.3, w: 4.4, h: 3.6, fill: { color: BG_MID }, shadow: cardShadow() });
  s.addShape(pres.shapes.RECTANGLE, { x: 5.2, y: 1.3, w: 4.4, h: 0.45, fill: { color: ACCENT } });
  s.addText("C — switch / if-else", { x: 5.4, y: 1.32, w: 4.0, h: 0.42, fontSize: 14, fontFace: TITLE_FONT, color: "FFFFFF", bold: true, margin: 0 });
  s.addText([
    { text: "switch(state) {", options: { color: ACCENT, fontSize: 10, fontFace: CODE_FONT, breakLine: true } },
    { text: "  case GREEN: ... break;", options: { color: FG, fontSize: 10, fontFace: CODE_FONT, breakLine: true } },
    { text: "  case YELLOW: ... break;", options: { color: FG, fontSize: 10, fontFace: CODE_FONT, breakLine: true } },
    { text: "  case ORANGE: ... break;", options: { color: FG, fontSize: 10, fontFace: CODE_FONT, breakLine: true } },
    { text: "  // forgot RED — compiles fine!", options: { color: YELLOW, fontSize: 10, fontFace: CODE_FONT, breakLine: true } },
    { text: "  // falls through to default", options: { color: YELLOW, fontSize: 10, fontFace: CODE_FONT, breakLine: true } },
    { text: "  // or worse: no default at all", options: { color: YELLOW, fontSize: 10, fontFace: CODE_FONT, breakLine: true } },
    { text: "}", options: { color: ACCENT, fontSize: 10, fontFace: CODE_FONT } },
  ], { x: 5.4, y: 1.9, w: 4.0, h: 2.8, valign: "top" });

  s.addText([
    { text: "Add a new enum variant? ", options: { color: FG_DIM, fontSize: 12, fontFace: BODY_FONT } },
    { text: "Rust compiler shows every match that needs updating.", options: { color: FG, fontSize: 12, fontFace: BODY_FONT, bold: true } },
    { text: " C silently ignores the gap.", options: { color: ACCENT, fontSize: 12, fontFace: BODY_FONT } },
  ], { x: 0.6, y: 5.05, w: 8.8, h: 0.45, margin: 0 });
}

// =====================================================================
// SLIDE 4 — Concurrency
// =====================================================================
{
  const s = pres.addSlide();
  s.background = { color: BG_DARK };

  s.addText("Concurrency Without Data Races", {
    x: 0.6, y: 0.3, w: 8.8, h: 0.7,
    fontSize: 32, fontFace: TITLE_FONT, color: FG, bold: true, margin: 0
  });
  s.addShape(pres.shapes.RECTANGLE, { x: 0.6, y: 0.95, w: 2.2, h: 0.04, fill: { color: ORANGE } });

  s.addShape(pres.shapes.RECTANGLE, { x: 0.4, y: 1.3, w: 4.4, h: 2.9, fill: { color: BG_MID }, shadow: cardShadow() });
  s.addShape(pres.shapes.RECTANGLE, { x: 0.4, y: 1.3, w: 4.4, h: 0.45, fill: { color: GREEN } });
  s.addText("Rust RTIC — Priority Ceiling Protocol", { x: 0.6, y: 1.32, w: 4.0, h: 0.42, fontSize: 12, fontFace: TITLE_FONT, color: BG_DARK, bold: true, margin: 0 });
  s.addText([
    { text: "#[shared]", options: { color: BLUE, fontSize: 10, fontFace: CODE_FONT, breakLine: true } },
    { text: "struct Shared {", options: { color: FG, fontSize: 10, fontFace: CODE_FONT, breakLine: true } },
    { text: "    alert: AlertState,", options: { color: FG, fontSize: 10, fontFace: CODE_FONT, breakLine: true } },
    { text: "    deadman: DeadmanSwitch,", options: { color: FG, fontSize: 10, fontFace: CODE_FONT, breakLine: true } },
    { text: "}", options: { color: FG, fontSize: 10, fontFace: CODE_FONT, breakLine: true } },
    { text: "", options: { fontSize: 5, breakLine: true } },
    { text: "ctx.shared.deadman.lock(|d| {", options: { color: GREEN, fontSize: 10, fontFace: CODE_FONT, breakLine: true } },
    { text: "    d.on_touch_interrupt()", options: { color: GREEN, fontSize: 10, fontFace: CODE_FONT, breakLine: true } },
    { text: "}); // deadlock-free by construction", options: { color: GREEN, fontSize: 10, fontFace: CODE_FONT } },
  ], { x: 0.6, y: 1.9, w: 4.0, h: 2.2, valign: "top" });

  s.addShape(pres.shapes.RECTANGLE, { x: 5.2, y: 1.3, w: 4.4, h: 2.9, fill: { color: BG_MID }, shadow: cardShadow() });
  s.addShape(pres.shapes.RECTANGLE, { x: 5.2, y: 1.3, w: 4.4, h: 0.45, fill: { color: ACCENT } });
  s.addText("C — manual critical sections", { x: 5.4, y: 1.32, w: 4.0, h: 0.42, fontSize: 12, fontFace: TITLE_FONT, color: "FFFFFF", bold: true, margin: 0 });
  s.addText([
    { text: "volatile int deadman_state;", options: { color: ACCENT, fontSize: 10, fontFace: CODE_FONT, breakLine: true } },
    { text: "volatile int alert_level;", options: { color: ACCENT, fontSize: 10, fontFace: CODE_FONT, breakLine: true } },
    { text: "// globals — any ISR can touch them", options: { color: FG_DIM, fontSize: 10, fontFace: CODE_FONT, breakLine: true } },
    { text: "", options: { fontSize: 5, breakLine: true } },
    { text: "__disable_irq();", options: { color: YELLOW, fontSize: 10, fontFace: CODE_FONT, breakLine: true } },
    { text: "deadman_state = GREEN;", options: { color: FG, fontSize: 10, fontFace: CODE_FONT, breakLine: true } },
    { text: "__enable_irq();", options: { color: YELLOW, fontSize: 10, fontFace: CODE_FONT, breakLine: true } },
    { text: "// forgot to protect alert_level?", options: { color: YELLOW, fontSize: 10, fontFace: CODE_FONT } },
  ], { x: 5.4, y: 1.9, w: 4.0, h: 2.2, valign: "top" });

  const kpY = 4.45;
  const kpW = 2.85;
  const points = [
    { title: "Declared at compile time", desc: "Shared resources are explicit in the type system", col: BLUE },
    { title: "Deadlock-free by math", desc: "Priority Ceiling Protocol — proven, not hoped for", col: GREEN },
    { title: "No global volatiles", desc: "Compiler rejects unprotected shared access", col: ORANGE },
  ];
  points.forEach((p, i) => {
    const px = 0.4 + i * (kpW + 0.25);
    s.addShape(pres.shapes.RECTANGLE, { x: px, y: kpY, w: kpW, h: 0.9, fill: { color: BG_MID }, shadow: cardShadow() });
    s.addShape(pres.shapes.RECTANGLE, { x: px, y: kpY, w: kpW, h: 0.06, fill: { color: p.col } });
    s.addText(p.title, { x: px + 0.15, y: kpY + 0.1, w: kpW - 0.3, h: 0.35, fontSize: 11, fontFace: BODY_FONT, color: FG, bold: true, margin: 0 });
    s.addText(p.desc, { x: px + 0.15, y: kpY + 0.45, w: kpW - 0.3, h: 0.35, fontSize: 9, fontFace: BODY_FONT, color: FG_DIM, margin: 0 });
  });
}

// =====================================================================
// SLIDE 5 — Testing
// =====================================================================
{
  const s = pres.addSlide();
  s.background = { color: BG_DARK };

  s.addText("First-Class Testing on the Host", {
    x: 0.6, y: 0.3, w: 8.8, h: 0.7,
    fontSize: 32, fontFace: TITLE_FONT, color: FG, bold: true, margin: 0
  });
  s.addShape(pres.shapes.RECTANGLE, { x: 0.6, y: 0.95, w: 2.2, h: 0.04, fill: { color: GREEN } });

  const stats = [
    { num: "53", label: "Tests", col: GREEN },
    { num: "5", label: "Categories", col: BLUE },
    { num: "0", label: "Failures", col: ACCENT },
    { num: "0s", label: "Runtime", col: ORANGE },
  ];
  stats.forEach((st, i) => {
    const sx = 0.4 + i * 2.4;
    s.addShape(pres.shapes.RECTANGLE, { x: sx, y: 1.25, w: 2.1, h: 1.3, fill: { color: BG_MID }, shadow: cardShadow() });
    s.addText(st.num, { x: sx, y: 1.3, w: 2.1, h: 0.75, fontSize: 40, fontFace: TITLE_FONT, color: st.col, bold: true, align: "center", margin: 0 });
    s.addText(st.label, { x: sx, y: 2.05, w: 2.1, h: 0.4, fontSize: 13, fontFace: BODY_FONT, color: FG_DIM, align: "center", margin: 0 });
  });

  s.addShape(pres.shapes.RECTANGLE, { x: 0.4, y: 2.85, w: 9.2, h: 2.5, fill: { color: BG_MID }, shadow: cardShadow() });
  s.addText("How: conditional compilation separates logic from hardware", {
    x: 0.6, y: 2.95, w: 8.8, h: 0.4, fontSize: 14, fontFace: BODY_FONT, color: BLUE, bold: true, margin: 0
  });
  s.addText([
    { text: "#![cfg_attr(not(test), no_std)]  ", options: { color: GREEN, fontSize: 10, fontFace: CODE_FONT, breakLine: true } },
    { text: "mod alert;                         // always compiled — pure logic", options: { color: FG, fontSize: 10, fontFace: CODE_FONT, breakLine: true } },
    { text: "#[cfg(not(test))] mod deadman;     // hardware — skipped on host", options: { color: FG_DIM, fontSize: 10, fontFace: CODE_FONT, breakLine: true } },
    { text: "#[cfg(not(test))] mod uart;        // hardware — skipped on host", options: { color: FG_DIM, fontSize: 10, fontFace: CODE_FONT, breakLine: true } },
    { text: "", options: { fontSize: 6, breakLine: true } },
    { text: "$ cargo test --target x86_64-pc-windows-msvc", options: { color: YELLOW, fontSize: 10, fontFace: CODE_FONT, breakLine: true } },
    { text: "running 53 tests ... ", options: { color: FG_DIM, fontSize: 10, fontFace: CODE_FONT } },
    { text: "test result: ok. 53 passed; 0 failed", options: { color: GREEN, fontSize: 10, fontFace: CODE_FONT } },
  ], { x: 0.6, y: 3.35, w: 8.8, h: 2.0, valign: "top" });

  s.addText("In C: testing embedded code typically requires flashing hardware or complex mocking frameworks. No built-in test runner.", {
    x: 0.6, y: 5.1, w: 8.8, h: 0.4, fontSize: 11, fontFace: BODY_FONT, color: FG_DIM, italic: true, margin: 0
  });
}

// =====================================================================
// SLIDE 6 — Overview (transition)
// =====================================================================
{
  const s = pres.addSlide();
  s.background = { color: BG_DARK };
  s.addShape(pres.shapes.RECTANGLE, { x: 0, y: 0, w: 10, h: 0.06, fill: { color: ACCENT } });

  s.addText("What You Get With Rust", {
    x: 0.6, y: 0.3, w: 8.8, h: 0.7,
    fontSize: 32, fontFace: TITLE_FONT, color: FG, bold: true, margin: 0
  });

  const items = [
    { icon: "01", title: "Zero-Cost Abstractions", desc: "Enums, pattern matching, iterators — same assembly as hand-written C, but type-safe. No runtime, no GC, no allocator.", col: GREEN },
    { icon: "02", title: "Compiler-Enforced Correctness", desc: "Ownership prevents memory bugs. Exhaustive match prevents missed states. Borrow checker prevents data races. All at compile time.", col: BLUE },
    { icon: "03", title: "Fearless Concurrency", desc: "RTIC's Priority Ceiling Protocol is mathematically deadlock-free. Shared resources are declared, not hoped-for-correct with volatile globals.", col: ORANGE },
    { icon: "04", title: "Cargo Ecosystem", desc: "Built-in package manager, test runner, doc generator, and cross-compilation. No Makefiles, no manual dependency management.", col: ACCENT },
  ];
  items.forEach((item, i) => {
    const iy = 1.2 + i * 1.05;
    s.addShape(pres.shapes.RECTANGLE, { x: 0.4, y: iy, w: 9.2, h: 0.9, fill: { color: BG_MID }, shadow: cardShadow() });
    s.addShape(pres.shapes.RECTANGLE, { x: 0.4, y: iy, w: 0.07, h: 0.9, fill: { color: item.col } });
    s.addText(item.icon, { x: 0.65, y: iy + 0.15, w: 0.55, h: 0.55, fontSize: 20, fontFace: TITLE_FONT, color: item.col, bold: true, align: "center", valign: "middle", margin: 0 });
    s.addText(item.title, { x: 1.35, y: iy + 0.08, w: 7.8, h: 0.38, fontSize: 16, fontFace: BODY_FONT, color: FG, bold: true, margin: 0 });
    s.addText(item.desc, { x: 1.35, y: iy + 0.46, w: 7.8, h: 0.38, fontSize: 11, fontFace: BODY_FONT, color: FG_DIM, margin: 0 });
  });

  s.addText('"If it compiles, it works" isn\'t just a meme — in embedded Rust, it\'s the engineering model.', {
    x: 0.6, y: 5.0, w: 8.8, h: 0.5, fontSize: 14, fontFace: BODY_FONT, color: ACCENT, italic: true, align: "center", margin: 0
  });
  s.addShape(pres.shapes.RECTANGLE, { x: 0, y: 5.565, w: 10, h: 0.06, fill: { color: ACCENT } });
}

// =====================================================================
// SLIDE 7 — Deep Dive: Zero-Cost Abstractions
// =====================================================================
{
  const s = pres.addSlide();
  s.background = { color: BG_DARK };

  s.addShape(pres.shapes.RECTANGLE, { x: 0, y: 0, w: 0.08, h: 5.625, fill: { color: GREEN } });
  s.addText("01", { x: 0.25, y: 0.25, w: 0.6, h: 0.5, fontSize: 24, fontFace: TITLE_FONT, color: GREEN, bold: true, margin: 0 });
  s.addText("Zero-Cost Abstractions", {
    x: 0.9, y: 0.25, w: 8.5, h: 0.5, fontSize: 28, fontFace: TITLE_FONT, color: FG, bold: true, margin: 0
  });
  s.addText("High-level Rust compiles to the same machine code as hand-optimized C", {
    x: 0.9, y: 0.75, w: 8.5, h: 0.35, fontSize: 13, fontFace: BODY_FONT, color: FG_DIM, italic: true, margin: 0
  });

  // Code card: enum + match → same as if/else chain
  s.addShape(pres.shapes.RECTANGLE, { x: 0.3, y: 1.3, w: 4.5, h: 3.5, fill: { color: BG_MID }, shadow: cardShadow() });
  s.addShape(pres.shapes.RECTANGLE, { x: 0.3, y: 1.3, w: 4.5, h: 0.4, fill: { color: GREEN } });
  s.addText("Rust — enum + pattern match", { x: 0.5, y: 1.3, w: 4.0, h: 0.4, fontSize: 12, fontFace: TITLE_FONT, color: BG_DARK, bold: true, margin: 0 });
  s.addText([
    { text: "pub fn update(&mut self, drowsy: bool)", options: { color: BLUE, fontSize: 9.5, fontFace: CODE_FONT, breakLine: true } },
    { text: "  -> AlertLevel {", options: { color: BLUE, fontSize: 9.5, fontFace: CODE_FONT, breakLine: true } },
    { text: "  if drowsy {", options: { color: FG, fontSize: 9.5, fontFace: CODE_FONT, breakLine: true } },
    { text: "    self.drowsy_count.saturating_add(1);", options: { color: FG, fontSize: 9.5, fontFace: CODE_FONT, breakLine: true } },
    { text: "    if self.drowsy_count >= 10 {", options: { color: FG, fontSize: 9.5, fontFace: CODE_FONT, breakLine: true } },
    { text: "      self.level = AlertLevel::Alert2;", options: { color: GREEN, fontSize: 9.5, fontFace: CODE_FONT, breakLine: true } },
    { text: "    } else if self.drowsy_count >= 5 {", options: { color: FG, fontSize: 9.5, fontFace: CODE_FONT, breakLine: true } },
    { text: "      self.level = AlertLevel::Alert1;", options: { color: GREEN, fontSize: 9.5, fontFace: CODE_FONT, breakLine: true } },
    { text: "    }", options: { color: FG, fontSize: 9.5, fontFace: CODE_FONT, breakLine: true } },
    { text: "  }", options: { color: FG, fontSize: 9.5, fontFace: CODE_FONT, breakLine: true } },
    { text: "  // No heap. No vtable. No GC.", options: { color: FG_DIM, fontSize: 9.5, fontFace: CODE_FONT, breakLine: true } },
    { text: "  // Compiles to identical ARM assembly", options: { color: FG_DIM, fontSize: 9.5, fontFace: CODE_FONT, breakLine: true } },
    { text: "  // as a hand-written C if/else chain.", options: { color: FG_DIM, fontSize: 9.5, fontFace: CODE_FONT } },
  ], { x: 0.5, y: 1.8, w: 4.1, h: 2.9, valign: "top" });

  // Code card: Option<T> → zero-size optimization
  s.addShape(pres.shapes.RECTANGLE, { x: 5.2, y: 1.3, w: 4.5, h: 3.5, fill: { color: BG_MID }, shadow: cardShadow() });
  s.addShape(pres.shapes.RECTANGLE, { x: 5.2, y: 1.3, w: 4.5, h: 0.4, fill: { color: GREEN } });
  s.addText("Rust — Option instead of NULL", { x: 5.4, y: 1.3, w: 4.0, h: 0.4, fontSize: 12, fontFace: TITLE_FONT, color: BG_DARK, bold: true, margin: 0 });
  s.addText([
    { text: "// EEG parser — from eeg_sensor.rs", options: { color: FG_DIM, fontSize: 9.5, fontFace: CODE_FONT, breakLine: true } },
    { text: "pub fn parse(line: &[u8])", options: { color: BLUE, fontSize: 9.5, fontFace: CODE_FONT, breakLine: true } },
    { text: "  -> Option<EegData> {", options: { color: BLUE, fontSize: 9.5, fontFace: CODE_FONT, breakLine: true } },
    { text: "  let s = from_utf8(line).ok()?;", options: { color: FG, fontSize: 9.5, fontFace: CODE_FONT, breakLine: true } },
    { text: "  let rest = s.strip_prefix(\"E,\")?;", options: { color: FG, fontSize: 9.5, fontFace: CODE_FONT, breakLine: true } },
    { text: "  let alpha = parts.next()?.parse()?;", options: { color: FG, fontSize: 9.5, fontFace: CODE_FONT, breakLine: true } },
    { text: "  Some(EegData { alpha, beta })", options: { color: GREEN, fontSize: 9.5, fontFace: CODE_FONT, breakLine: true } },
    { text: "}", options: { color: BLUE, fontSize: 9.5, fontFace: CODE_FONT, breakLine: true } },
    { text: "", options: { fontSize: 5, breakLine: true } },
    { text: "// ? operator propagates None", options: { color: FG_DIM, fontSize: 9.5, fontFace: CODE_FONT, breakLine: true } },
    { text: "// No null pointer. No sentinel value.", options: { color: FG_DIM, fontSize: 9.5, fontFace: CODE_FONT, breakLine: true } },
    { text: "// Zero overhead — same as C return -1", options: { color: FG_DIM, fontSize: 9.5, fontFace: CODE_FONT } },
  ], { x: 5.4, y: 1.8, w: 4.1, h: 2.9, valign: "top" });

  s.addText("Every abstraction (enums, Option, iterators, closures) compiles away completely. You get safety AND performance.", {
    x: 0.5, y: 5.05, w: 9.0, h: 0.4, fontSize: 12, fontFace: BODY_FONT, color: GREEN, bold: true, margin: 0
  });
}

// =====================================================================
// SLIDE 8 — Deep Dive: Compiler-Enforced Correctness
// =====================================================================
{
  const s = pres.addSlide();
  s.background = { color: BG_DARK };

  s.addShape(pres.shapes.RECTANGLE, { x: 0, y: 0, w: 0.08, h: 5.625, fill: { color: BLUE } });
  s.addText("02", { x: 0.25, y: 0.25, w: 0.6, h: 0.5, fontSize: 24, fontFace: TITLE_FONT, color: BLUE, bold: true, margin: 0 });
  s.addText("Compiler-Enforced Correctness", {
    x: 0.9, y: 0.25, w: 8.5, h: 0.5, fontSize: 28, fontFace: TITLE_FONT, color: FG, bold: true, margin: 0
  });
  s.addText("Bugs that are runtime surprises in C become compile errors in Rust", {
    x: 0.9, y: 0.75, w: 8.5, h: 0.35, fontSize: 13, fontFace: BODY_FONT, color: FG_DIM, italic: true, margin: 0
  });

  // Left: type system prevents wrong values
  s.addShape(pres.shapes.RECTANGLE, { x: 0.3, y: 1.3, w: 4.5, h: 1.8, fill: { color: BG_MID }, shadow: cardShadow() });
  s.addShape(pres.shapes.RECTANGLE, { x: 0.3, y: 1.3, w: 0.06, h: 1.8, fill: { color: BLUE } });
  s.addText("Type system prevents wrong values", { x: 0.55, y: 1.35, w: 4.0, h: 0.35, fontSize: 13, fontFace: BODY_FONT, color: FG, bold: true, margin: 0 });
  s.addText([
    { text: "enum PowerMode { Low, Full }", options: { color: BLUE, fontSize: 9.5, fontFace: CODE_FONT, breakLine: true } },
    { text: "fn apply_clock(mode: PowerMode) {..}", options: { color: FG, fontSize: 9.5, fontFace: CODE_FONT, breakLine: true } },
    { text: "// Can ONLY pass Low or Full.", options: { color: FG_DIM, fontSize: 9.5, fontFace: CODE_FONT, breakLine: true } },
    { text: "// No accidental apply_clock(42).", options: { color: GREEN, fontSize: 9.5, fontFace: CODE_FONT, breakLine: true } },
    { text: "// No #define LOW 0 / #define FULL 1", options: { color: GREEN, fontSize: 9.5, fontFace: CODE_FONT } },
  ], { x: 0.55, y: 1.7, w: 4.1, h: 1.3, valign: "top" });

  // Right: borrow checker
  s.addShape(pres.shapes.RECTANGLE, { x: 5.2, y: 1.3, w: 4.5, h: 1.8, fill: { color: BG_MID }, shadow: cardShadow() });
  s.addShape(pres.shapes.RECTANGLE, { x: 5.2, y: 1.3, w: 0.06, h: 1.8, fill: { color: BLUE } });
  s.addText("Borrow checker prevents aliasing", { x: 5.45, y: 1.35, w: 4.0, h: 0.35, fontSize: 13, fontFace: BODY_FONT, color: FG, bold: true, margin: 0 });
  s.addText([
    { text: "let buf: &'static mut [u8; 128];", options: { color: FG, fontSize: 9.5, fontFace: CODE_FONT, breakLine: true } },
    { text: "EegSensor::init_dma(buf);", options: { color: FG, fontSize: 9.5, fontFace: CODE_FONT, breakLine: true } },
    { text: "// buf is MOVED into init_dma.", options: { color: FG_DIM, fontSize: 9.5, fontFace: CODE_FONT, breakLine: true } },
    { text: "// Using buf again? COMPILE ERROR.", options: { color: GREEN, fontSize: 9.5, fontFace: CODE_FONT, breakLine: true } },
    { text: "// DMA and CPU can't alias the buffer.", options: { color: GREEN, fontSize: 9.5, fontFace: CODE_FONT } },
  ], { x: 5.45, y: 1.7, w: 4.1, h: 1.3, valign: "top" });

  // Bottom: de-escalation match
  s.addShape(pres.shapes.RECTANGLE, { x: 0.3, y: 3.35, w: 9.4, h: 2.0, fill: { color: BG_MID }, shadow: cardShadow() });
  s.addShape(pres.shapes.RECTANGLE, { x: 0.3, y: 3.35, w: 9.4, h: 0.4, fill: { color: BLUE } });
  s.addText("Exhaustive match on alert de-escalation — from alert.rs", { x: 0.5, y: 3.36, w: 9.0, h: 0.38, fontSize: 12, fontFace: TITLE_FONT, color: "FFFFFF", bold: true, margin: 0 });
  s.addText([
    { text: "match self.level {                                    // In C:", options: { color: BLUE, fontSize: 10, fontFace: CODE_FONT, breakLine: true } },
    { text: "    AlertLevel::Alert2 => self.level = Alert1,        // if (level == 2) level = 1;", options: { color: FG, fontSize: 10, fontFace: CODE_FONT, breakLine: true } },
    { text: "    AlertLevel::Alert1 => self.level = None,          // else if (level == 1) level = 0;", options: { color: FG, fontSize: 10, fontFace: CODE_FONT, breakLine: true } },
    { text: "    AlertLevel::None   => {}                          // else { /* noop */ }", options: { color: FG, fontSize: 10, fontFace: CODE_FONT, breakLine: true } },
    { text: "}", options: { color: BLUE, fontSize: 10, fontFace: CODE_FONT, breakLine: true } },
    { text: "// Add AlertLevel::Alert3? Rust forces you to handle it here. C won't even warn.", options: { color: YELLOW, fontSize: 10, fontFace: CODE_FONT } },
  ], { x: 0.5, y: 3.8, w: 9.0, h: 1.45, valign: "top" });
}

// =====================================================================
// SLIDE 9 — Deep Dive: Fearless Concurrency
// =====================================================================
{
  const s = pres.addSlide();
  s.background = { color: BG_DARK };

  s.addShape(pres.shapes.RECTANGLE, { x: 0, y: 0, w: 0.08, h: 5.625, fill: { color: ORANGE } });
  s.addText("03", { x: 0.25, y: 0.25, w: 0.6, h: 0.5, fontSize: 24, fontFace: TITLE_FONT, color: ORANGE, bold: true, margin: 0 });
  s.addText("Fearless Concurrency", {
    x: 0.9, y: 0.25, w: 8.5, h: 0.5, fontSize: 28, fontFace: TITLE_FONT, color: FG, bold: true, margin: 0
  });
  s.addText("The classic embedded freeze bug — and how Rust's architecture prevents it", {
    x: 0.9, y: 0.75, w: 8.5, h: 0.35, fontSize: 13, fontFace: BODY_FONT, color: FG_DIM, italic: true, margin: 0
  });

  // Top: the ISR split pattern
  s.addShape(pres.shapes.RECTANGLE, { x: 0.3, y: 1.25, w: 9.4, h: 2.4, fill: { color: BG_MID }, shadow: cardShadow() });
  s.addShape(pres.shapes.RECTANGLE, { x: 0.3, y: 1.25, w: 9.4, h: 0.4, fill: { color: ORANGE } });
  s.addText("ISR split: fast acknowledge + deferred heavy work", { x: 0.5, y: 1.26, w: 9.0, h: 0.38, fontSize: 12, fontFace: TITLE_FONT, color: BG_DARK, bold: true, margin: 0 });
  s.addText([
    { text: "// EXTI15 ISR — priority 3, runs in ~100 ns", options: { color: FG_DIM, fontSize: 9.5, fontFace: CODE_FONT, breakLine: true } },
    { text: "#[task(binds = EXTI15_10, priority = 3)]", options: { color: BLUE, fontSize: 9.5, fontFace: CODE_FONT, breakLine: true } },
    { text: "fn exti15_touch(_ctx: ..) {", options: { color: FG, fontSize: 9.5, fontFace: CODE_FONT, breakLine: true } },
    { text: "    TouchScreen::clear_exti_pending();  // clear HW flag", options: { color: FG, fontSize: 9.5, fontFace: CODE_FONT, breakLine: true } },
    { text: "    handle_touch::spawn().ok();          // defer to priority 1", options: { color: GREEN, fontSize: 9.5, fontFace: CODE_FONT, breakLine: true } },
    { text: "}", options: { color: FG, fontSize: 9.5, fontFace: CODE_FONT, breakLine: true } },
    { text: "", options: { fontSize: 4, breakLine: true } },
    { text: "// handle_touch — priority 1, ~800 us of blocking I2C", options: { color: FG_DIM, fontSize: 9.5, fontFace: CODE_FONT, breakLine: true } },
    { text: "#[task(shared = [deadman], capacity = 2)]", options: { color: BLUE, fontSize: 9.5, fontFace: CODE_FONT, breakLine: true } },
    { text: "fn handle_touch(mut ctx: ..) {", options: { color: FG, fontSize: 9.5, fontFace: CODE_FONT, breakLine: true } },
    { text: "    ctx.shared.deadman.lock(|d| d.on_touch_interrupt());", options: { color: GREEN, fontSize: 9.5, fontFace: CODE_FONT, breakLine: true } },
    { text: "}  // UART DMA keeps buffering during this — nothing freezes", options: { color: ORANGE, fontSize: 9.5, fontFace: CODE_FONT } },
  ], { x: 0.5, y: 1.7, w: 9.0, h: 1.85, valign: "top" });

  // Bottom cards
  s.addShape(pres.shapes.RECTANGLE, { x: 0.3, y: 3.9, w: 4.5, h: 1.4, fill: { color: BG_MID }, shadow: cardShadow() });
  s.addShape(pres.shapes.RECTANGLE, { x: 0.3, y: 3.9, w: 0.06, h: 1.4, fill: { color: GREEN } });
  s.addText("Why this can't freeze", { x: 0.55, y: 3.95, w: 4.0, h: 0.3, fontSize: 13, fontFace: BODY_FONT, color: GREEN, bold: true, margin: 0 });
  s.addText([
    { text: "EXTI15 (prio 3) exits in 100 ns", options: { bullet: true, breakLine: true, fontSize: 11, fontFace: BODY_FONT, color: FG } },
    { text: "I2C runs at priority 1 — UART can preempt", options: { bullet: true, breakLine: true, fontSize: 11, fontFace: BODY_FONT, color: FG } },
    { text: "DMA buffers data independently of CPU", options: { bullet: true, breakLine: true, fontSize: 11, fontFace: BODY_FONT, color: FG } },
    { text: "All I2C loops have hard 100K-cycle timeouts", options: { bullet: true, fontSize: 11, fontFace: BODY_FONT, color: FG } },
  ], { x: 0.55, y: 4.25, w: 4.0, h: 1.0, valign: "top" });

  s.addShape(pres.shapes.RECTANGLE, { x: 5.2, y: 3.9, w: 4.5, h: 1.4, fill: { color: BG_MID }, shadow: cardShadow() });
  s.addShape(pres.shapes.RECTANGLE, { x: 5.2, y: 3.9, w: 0.06, h: 1.4, fill: { color: ACCENT } });
  s.addText("C equivalent would need", { x: 5.45, y: 3.95, w: 4.0, h: 0.3, fontSize: 13, fontFace: BODY_FONT, color: ACCENT, bold: true, margin: 0 });
  s.addText([
    { text: "Manual NVIC priority configuration", options: { bullet: true, breakLine: true, fontSize: 11, fontFace: BODY_FONT, color: FG } },
    { text: "Hand-written deferred work queues", options: { bullet: true, breakLine: true, fontSize: 11, fontFace: BODY_FONT, color: FG } },
    { text: "volatile + __disable_irq() discipline", options: { bullet: true, breakLine: true, fontSize: 11, fontFace: BODY_FONT, color: FG } },
    { text: "Hope nobody breaks the convention", options: { bullet: true, fontSize: 11, fontFace: BODY_FONT, color: YELLOW } },
  ], { x: 5.45, y: 4.25, w: 4.0, h: 1.0, valign: "top" });
}

// =====================================================================
// SLIDE 10 — Deep Dive: Cargo Ecosystem
// =====================================================================
{
  const s = pres.addSlide();
  s.background = { color: BG_DARK };

  s.addShape(pres.shapes.RECTANGLE, { x: 0, y: 0, w: 0.08, h: 5.625, fill: { color: ACCENT } });
  s.addText("04", { x: 0.25, y: 0.25, w: 0.6, h: 0.5, fontSize: 24, fontFace: TITLE_FONT, color: ACCENT, bold: true, margin: 0 });
  s.addText("Cargo Ecosystem", {
    x: 0.9, y: 0.25, w: 8.5, h: 0.5, fontSize: 28, fontFace: TITLE_FONT, color: FG, bold: true, margin: 0
  });
  s.addText("One tool replaces Make, CMake, pkg-config, Unity, and your CI scripts", {
    x: 0.9, y: 0.75, w: 8.5, h: 0.35, fontSize: 13, fontFace: BODY_FONT, color: FG_DIM, italic: true, margin: 0
  });

  // Left: Cargo.toml
  s.addShape(pres.shapes.RECTANGLE, { x: 0.3, y: 1.25, w: 4.5, h: 3.2, fill: { color: BG_MID }, shadow: cardShadow() });
  s.addShape(pres.shapes.RECTANGLE, { x: 0.3, y: 1.25, w: 4.5, h: 0.4, fill: { color: ACCENT } });
  s.addText("Cargo.toml — entire build config", { x: 0.5, y: 1.26, w: 4.0, h: 0.38, fontSize: 12, fontFace: TITLE_FONT, color: "FFFFFF", bold: true, margin: 0 });
  s.addText([
    { text: "[dependencies]", options: { color: ACCENT, fontSize: 9.5, fontFace: CODE_FONT, breakLine: true } },
    { text: "cortex-m-rtic = \"1.1.4\"", options: { color: FG, fontSize: 9.5, fontFace: CODE_FONT, breakLine: true } },
    { text: "stm32f4xx-hal = { version = \"0.21.0\",", options: { color: FG, fontSize: 9.5, fontFace: CODE_FONT, breakLine: true } },
    { text: "  features = [\"stm32f429\"] }", options: { color: FG, fontSize: 9.5, fontFace: CODE_FONT, breakLine: true } },
    { text: "heapless = \"0.8\"", options: { color: FG, fontSize: 9.5, fontFace: CODE_FONT, breakLine: true } },
    { text: "rtt-target = \"0.5\"", options: { color: FG, fontSize: 9.5, fontFace: CODE_FONT, breakLine: true } },
    { text: "", options: { fontSize: 5, breakLine: true } },
    { text: "[profile.release]", options: { color: ACCENT, fontSize: 9.5, fontFace: CODE_FONT, breakLine: true } },
    { text: "lto = true", options: { color: FG, fontSize: 9.5, fontFace: CODE_FONT, breakLine: true } },
    { text: "opt-level = \"s\"  # size-optimized", options: { color: FG, fontSize: 9.5, fontFace: CODE_FONT, breakLine: true } },
    { text: "", options: { fontSize: 5, breakLine: true } },
    { text: "// That's it. No Makefile. No CMake.", options: { color: GREEN, fontSize: 9.5, fontFace: CODE_FONT } },
  ], { x: 0.5, y: 1.7, w: 4.1, h: 2.7, valign: "top" });

  // Right: one command does everything
  s.addShape(pres.shapes.RECTANGLE, { x: 5.2, y: 1.25, w: 4.5, h: 3.2, fill: { color: BG_MID }, shadow: cardShadow() });
  s.addShape(pres.shapes.RECTANGLE, { x: 5.2, y: 1.25, w: 4.5, h: 0.4, fill: { color: GREEN } });
  s.addText("One toolchain, everything built in", { x: 5.4, y: 1.26, w: 4.0, h: 0.38, fontSize: 12, fontFace: TITLE_FONT, color: BG_DARK, bold: true, margin: 0 });
  s.addText([
    { text: "$ cargo build --release", options: { color: YELLOW, fontSize: 9.5, fontFace: CODE_FONT, breakLine: true } },
    { text: "  # cross-compile to thumbv7em-none-eabihf", options: { color: FG_DIM, fontSize: 9.5, fontFace: CODE_FONT, breakLine: true } },
    { text: "", options: { fontSize: 4, breakLine: true } },
    { text: "$ cargo test --target x86_64-...", options: { color: YELLOW, fontSize: 9.5, fontFace: CODE_FONT, breakLine: true } },
    { text: "  # run 53 tests on host — no HW needed", options: { color: FG_DIM, fontSize: 9.5, fontFace: CODE_FONT, breakLine: true } },
    { text: "", options: { fontSize: 4, breakLine: true } },
    { text: "$ cargo doc --open", options: { color: YELLOW, fontSize: 9.5, fontFace: CODE_FONT, breakLine: true } },
    { text: "  # auto-generate HTML API docs", options: { color: FG_DIM, fontSize: 9.5, fontFace: CODE_FONT, breakLine: true } },
    { text: "", options: { fontSize: 4, breakLine: true } },
    { text: "$ cargo clippy", options: { color: YELLOW, fontSize: 9.5, fontFace: CODE_FONT, breakLine: true } },
    { text: "  # 600+ lint rules — catches bugs", options: { color: FG_DIM, fontSize: 9.5, fontFace: CODE_FONT, breakLine: true } },
    { text: "", options: { fontSize: 4, breakLine: true } },
    { text: "// C needs: Make/CMake + pkg-config +", options: { color: ACCENT, fontSize: 9.5, fontFace: CODE_FONT, breakLine: true } },
    { text: "// Unity/CTest + Doxygen + cppcheck", options: { color: ACCENT, fontSize: 9.5, fontFace: CODE_FONT } },
  ], { x: 5.4, y: 1.7, w: 4.1, h: 2.7, valign: "top" });

  // Bottom comparison bar
  s.addShape(pres.shapes.RECTANGLE, { x: 0.3, y: 4.7, w: 9.4, h: 0.7, fill: { color: BG_MID }, shadow: cardShadow() });
  s.addText([
    { text: "C embedded project setup: ", options: { color: FG_DIM, fontSize: 12, fontFace: BODY_FONT } },
    { text: "Makefile + linker script + startup.s + syscalls.c + HAL config wizard + manual SVD parsing", options: { color: ACCENT, fontSize: 12, fontFace: BODY_FONT } },
  ], { x: 0.5, y: 4.72, w: 9.0, h: 0.32, margin: 0 });
  s.addText([
    { text: "Rust embedded project setup: ", options: { color: FG_DIM, fontSize: 12, fontFace: BODY_FONT } },
    { text: "cargo init + add dependencies to Cargo.toml", options: { color: GREEN, fontSize: 12, fontFace: BODY_FONT, bold: true } },
  ], { x: 0.5, y: 5.04, w: 9.0, h: 0.32, margin: 0 });
}

// ---- write ----
pres.writeFile({ fileName: "C:\\Users\\USER\\EEG_drowsiness_detection\\Rust_vs_C_Embedded.pptx" })
  .then(() => console.log("Created: Rust_vs_C_Embedded.pptx (10 slides)"))
  .catch(err => console.error(err));
