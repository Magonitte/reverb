// `npm run e2e:app`: E2E no app Tauri real (WebdriverIO + tauri-driver), F07 T10/T11.
//
// 1. compila o app (`tauri build --debug --no-bundle`: embute o `dist/` e mantém `debug_assertions`,
//    que habilita REVERB_DATA_DIR/REVERB_TOOLS_DIR — um `cargo build` puro carregaria o devUrl);
// 2. garante as ferramentas reais em `.test-tools/` e o msedgedriver da versão do WebView2;
// 3. cria diretórios temporários de dados e de saída (os testes nunca tocam nas pastas do usuário);
// 4. roda o WebdriverIO, que sobe o tauri-driver.
import { spawnSync } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

const root = resolve(import.meta.dirname, "..");
const isWindows = process.platform === "win32";
const exe = (name) => (isWindows ? `${name}.exe` : name);

/** Roda um comando com argumentos como lista; sai com o código dele em caso de falha. */
function run(command, args, options = {}) {
  console.log(`\n=== ${command} ${args.join(" ")}`);
  const result = spawnSync(command, args, {
    stdio: "inherit",
    cwd: root,
    shell: isWindows && /^(npx|npm)$/.test(command),
    ...options,
  });
  if (result.status !== 0) {
    console.error(`e2e:app: falha em "${command} ${args.join(" ")}"`);
    process.exit(result.status ?? 1);
  }
}

/** Versão do WebView2 instalada (o msedgedriver precisa ter a mesma versão principal). */
function webview2Version() {
  const key =
    "HKLM\\SOFTWARE\\WOW6432Node\\Microsoft\\EdgeUpdate\\Clients\\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}";
  const out = spawnSync("reg", ["query", key, "/v", "pv"], { encoding: "utf8" });
  const match = /pv\s+REG_SZ\s+(\S+)/.exec(out.stdout ?? "");
  if (!match) throw new Error("e2e:app: WebView2 Runtime não encontrado no registro");
  return match[1];
}

/** Baixa o msedgedriver da Microsoft para a versão do WebView2 (idempotente). */
async function ensureEdgeDriver() {
  const version = webview2Version();
  const cached = join(toolsDir, "msedgedriver", version, "msedgedriver.exe");
  if (existsSync(cached)) return cached;
  const dir = join(root, ".test-tools", "msedgedriver", version);
  const driver = join(dir, "msedgedriver.exe");
  if (existsSync(driver)) return driver;
  mkdirSync(dir, { recursive: true });
  const url = `https://msedgedriver.microsoft.com/${version}/edgedriver_win64.zip`;
  console.log(`\n=== baixando o msedgedriver ${version}\n${url}`);
  const response = await fetch(url);
  if (!response.ok)
    throw new Error(`e2e:app: download do msedgedriver falhou (${response.status})`);
  const zip = join(dir, "edgedriver.zip");
  writeFileSync(zip, Buffer.from(await response.arrayBuffer()));
  // `tar` pode ser o GNU tar do Git Bash (não lê .zip); o Expand-Archive existe em todo Windows.
  const quote = (path) => `'${path.replaceAll("'", "''")}'`;
  const result = spawnSync(
    "powershell",
    [
      "-NoProfile",
      "-Command",
      `Expand-Archive -LiteralPath ${quote(zip)} -DestinationPath ${quote(dir)} -Force`,
    ],
    { stdio: "inherit" },
  );
  if (result.status !== 0 || !existsSync(driver)) {
    throw new Error("e2e:app: não foi possível extrair o msedgedriver");
  }
  rmSync(zip);
  return driver;
}

/** Acha o tauri-driver no PATH ou em ~/.cargo/bin. */
function tauriDriverPath() {
  const cargoBin = join(
    process.env.CARGO_HOME ?? join(process.env.USERPROFILE ?? "", ".cargo"),
    "bin",
  );
  const candidate = join(cargoBin, exe("tauri-driver"));
  if (existsSync(candidate)) return candidate;
  const found = spawnSync(isWindows ? "where" : "which", ["tauri-driver"], { encoding: "utf8" });
  if (found.status === 0) return found.stdout.split(/\r?\n/)[0];
  throw new Error("e2e:app: tauri-driver não encontrado (cargo install tauri-driver --locked)");
}

if (!isWindows) {
  console.error(
    "e2e:app: por enquanto só roda no Windows (msedgedriver); o Linux é validado pelo CI",
  );
  process.exit(1);
}

const toolsDir = process.env.REVERB_TEST_TOOLS_DIR ?? join(root, ".test-tools");
if (!existsSync(join(toolsDir, "manifest.json"))) run("node", ["scripts/test-prepare.mjs"]);

run("npx", ["tauri", "build", "--debug", "--no-bundle"]);
run("cargo", ["build", "-p", "reverb-cli"]);

const nativeDriver = await ensureEdgeDriver();
const tauriDriver = tauriDriverPath();

const scratch = mkdtempSync(join(tmpdir(), "reverb-e2e-app-"));
const dataDir = join(scratch, "data");
const outputDir = join(scratch, "musicas");
mkdirSync(dataDir, { recursive: true });
mkdirSync(outputDir, { recursive: true });

// Configura o app antes de abrir: pasta de saída temporária e onboarding concluído.
const cli = join(root, "target", "debug", exe("reverb-cli"));
const setting = (key, json) => run(cli, ["--data-dir", dataDir, "settings", "set", key, json]);
setting("outputDir", JSON.stringify(outputDir));
setting("onboardingCompleted", "true");

const env = {
  ...process.env,
  REVERB_DATA_DIR: dataDir,
  REVERB_TOOLS_DIR: toolsDir,
  REVERB_E2E_APP: join(root, "target", "debug", exe("reverb")),
  REVERB_E2E_OUTPUT_DIR: outputDir,
  REVERB_E2E_NATIVE_DRIVER: nativeDriver,
  REVERB_E2E_TAURI_DRIVER: tauriDriver,
};

const result = spawnSync("npx", ["wdio", "run", "tests/e2e-app/wdio.conf.mjs"], {
  stdio: "inherit",
  cwd: root,
  shell: true,
  env,
});

// Limpa só o que este script criou, e só se a execução terminou (o app já fechou).
try {
  rmSync(scratch, { recursive: true, force: true });
} catch (error) {
  console.warn(`e2e:app: não foi possível apagar ${scratch}: ${error.message}`);
}
process.exit(result.status ?? 1);
