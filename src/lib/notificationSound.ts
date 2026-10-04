import glass from "@/assets/sounds/glass.wav";
import marimba from "@/assets/sounds/marimba.wav";
import sparkle from "@/assets/sounds/sparkle.wav";
import { useNotificationPreferences } from "@/stores/notificationPreferences";
const SOUNDS = { glass, marimba, sparkle };
let audio: HTMLAudioElement | null = null;
let stopTimer: ReturnType<typeof setTimeout> | undefined;
let lastPlayed = 0;
export async function playNotificationSound(preview = false): Promise<void> {
  const p = useNotificationPreferences.getState().preferences;
  if (p.sound === "silent" || !p.volume) return;
  if (!preview && Date.now() - lastPlayed < 2000) return;
  const source = p.sound === "custom" ? p.customData : SOUNDS[p.sound];
  if (!source) return;
  audio?.pause();
  clearTimeout(stopTimer);
  audio = new Audio(source);
  audio.volume = p.volume;
  await audio.play();
  lastPlayed = Date.now();
  stopTimer = setTimeout(() => audio?.pause(), 5000);
}
export async function readNotificationAudio(file: File): Promise<string> {
  if (file.size > 2 * 1024 * 1024 || !/\.(mp3|wav|ogg|m4a)$/i.test(file.name))
    throw new Error("audio-file");
  const AudioContextClass = window.AudioContext;
  const context = new AudioContextClass();
  try {
    await context.decodeAudioData(await file.arrayBuffer());
  } finally {
    await context.close();
  }
  const encoded = await new Promise<string>((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => resolve(String(reader.result));
    reader.onerror = reject;
    reader.readAsDataURL(
      new Blob([file], {
        type: file.type.startsWith("audio/")
          ? file.type
          : ({ mp3: "audio/mpeg", wav: "audio/wav", ogg: "audio/ogg", m4a: "audio/mp4" }[
              file.name.split(".").pop()?.toLowerCase() ?? ""
            ] ?? "audio/wav"),
      }),
    );
  });
  return encoded;
}
