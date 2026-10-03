// Configuração do WebdriverIO para o app Tauri real. Quem a prepara (variáveis REVERB_E2E_*,
// dados temporários, build) é `scripts/e2e-app.mjs`; rode sempre por `npm run e2e:app`.
import { spawn } from "node:child_process";
import net from "node:net";

const required = (name) => {
  const value = process.env[name];
  if (!value) throw new Error(`${name} não definido: rode "npm run e2e:app"`);
  return value;
};

const PORT = 4444;
let tauriDriver = null;

/** Espera a porta do tauri-driver aceitar conexões. */
function waitForPort(port, timeoutMs = 20_000) {
  const deadline = Date.now() + timeoutMs;
  return new Promise((resolve, reject) => {
    const attempt = () => {
      const socket = net.connect(port, "127.0.0.1");
      socket.once("connect", () => {
        socket.destroy();
        resolve();
      });
      socket.once("error", () => {
        socket.destroy();
        if (Date.now() > deadline) reject(new Error("tauri-driver não subiu a tempo"));
        else setTimeout(attempt, 250);
      });
    };
    attempt();
  });
}

export const config = {
  runner: "local",
  hostname: "127.0.0.1",
  port: PORT,
  path: "/",
  specs: ["./specs/**/*.mjs"],
  // Um app por vez: os dois specs dividem a pasta de dados temporária.
  maxInstances: 1,
  capabilities: [
    {
      maxInstances: 1,
      "tauri:options": { application: required("REVERB_E2E_APP") },
    },
  ],
  reporters: ["spec"],
  framework: "mocha",
  mochaOpts: { ui: "bdd", timeout: 240_000 },
  logLevel: "warn",
  waitforTimeout: 30_000,

  // Cada sessão abre o app de novo; o tauri-driver precisa estar de pé antes dela.
  beforeSession: async () => {
    tauriDriver = spawn(
      required("REVERB_E2E_TAURI_DRIVER"),
      ["--native-driver", required("REVERB_E2E_NATIVE_DRIVER")],
      { stdio: [null, process.stdout, process.stderr] },
    );
    tauriDriver.on("error", (error) => {
      console.error("tauri-driver falhou ao iniciar:", error);
      process.exit(1);
    });
    await waitForPort(PORT);
  },
  afterSession: () => {
    tauriDriver?.kill();
    tauriDriver = null;
  },
};
