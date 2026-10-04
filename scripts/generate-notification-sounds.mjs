import { mkdirSync, writeFileSync } from "node:fs";
const rate = 22050,
  length = 1.6,
  count = Math.floor(rate * length);
mkdirSync("src/assets/sounds", { recursive: true });
const sounds = {
  glass: [
    [0, 880, 0.45],
    [0.16, 1320, 0.32],
    [0.32, 1760, 0.22],
  ],
  marimba: [
    [0, 523.25, 0.6],
    [0.2, 659.25, 0.5],
    [0.4, 783.99, 0.4],
  ],
  sparkle: [
    [0, 659.25, 0.4],
    [0.14, 987.77, 0.4],
    [0.28, 1318.51, 0.35],
    [0.42, 1567.98, 0.3],
  ],
};
for (const [name, notes] of Object.entries(sounds)) {
  const wave = Buffer.alloc(44 + count * 2);
  wave.write("RIFF");
  wave.writeUInt32LE(36 + count * 2, 4);
  wave.write("WAVEfmt ", 8);
  wave.writeUInt32LE(16, 16);
  wave.writeUInt16LE(1, 20);
  wave.writeUInt16LE(1, 22);
  wave.writeUInt32LE(rate, 24);
  wave.writeUInt32LE(rate * 2, 28);
  wave.writeUInt16LE(2, 32);
  wave.writeUInt16LE(16, 34);
  wave.write("data", 36);
  wave.writeUInt32LE(count * 2, 40);
  for (let i = 0; i < count; i++) {
    let value = 0;
    const t = i / rate;
    for (const [start, freq, gain] of notes) {
      const dt = t - start;
      if (dt < 0) continue;
      const env = Math.min(1, dt / 0.012) * Math.exp(-dt * 5);
      value +=
        gain *
        env *
        (Math.sin(2 * Math.PI * freq * dt) + 0.15 * Math.sin(2 * Math.PI * freq * 2 * dt));
    }
    value *= Math.min(1, (length - t) / 0.08) * 0.35;
    wave.writeInt16LE(Math.round(Math.max(-1, Math.min(1, value)) * 32767), 44 + i * 2);
  }
  writeFileSync(`src/assets/sounds/${name}.wav`, wave);
}
