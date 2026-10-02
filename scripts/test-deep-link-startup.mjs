// F12 T10b: launch a closed app with a link, inspect its isolated persistent queue.
import assert from "node:assert/strict";
import { spawn, spawnSync } from "node:child_process";
import { mkdirSync, mkdtempSync, realpathSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

export async function testDeepLinkStartup(app, cli, toolsDir) {
  if (process.platform === "win32") {
    const running = spawnSync(
      "powershell.exe",
      ["-NoProfile", "-Command", "@(Get-Process -Name reverb -ErrorAction SilentlyContinue).Count"],
      { encoding: "utf8", windowsHide: true },
    );
    assert.equal(running.status, 0, "could not check existing Reverb instances");
    assert.equal(
      Number(running.stdout.trim()),
      0,
      "close existing Reverb instances before running the isolated desktop tests",
    );
  }
  const scratch = realpathSync.native(mkdtempSync(join(tmpdir(), "reverb-deeplink-")));
  const data = join(scratch, "data");
  mkdirSync(data);
  const command = (args) => {
    const result = spawnSync(cli, ["--data-dir", data, ...args], {
      encoding: "utf8",
      windowsHide: true,
    });
    assert.equal(result.status, 0, result.stderr);
    return result.stdout;
  };
  command(["settings", "set", "outputDir", JSON.stringify(join(scratch, "music"))]);
  command(["settings", "set", "onboardingCompleted", "true"]);
  command(["settings", "set", "autoUpdateTools", "false"]);
  command(["settings", "set", "autoCheckAppUpdates", "false"]);
  const child = spawn(
    app,
    ["reverb://add?url=https%3A%2F%2Fwww.youtube.com%2Fwatch%3Fv%3DjNQXAC9IVRw"],
    {
      windowsHide: true,
      stdio: "ignore",
      env: { ...process.env, REVERB_DATA_DIR: data, REVERB_TOOLS_DIR: toolsDir },
    },
  );
  let failure;
  child.on("error", (error) => {
    failure = error;
  });
  try {
    const deadline = Date.now() + 60000;
    let jobs = [];
    while (Date.now() < deadline) {
      if (failure) throw failure;
      assert.equal(child.exitCode, null, "cold-launch app exited before creating its job");
      jobs = JSON.parse(command(["jobs", "list", "--json"]));
      if (jobs.length) break;
      await new Promise((resolve) => setTimeout(resolve, 500));
    }
    assert.equal(jobs.length, 1, "cold launch must create exactly one job");
    assert.equal(jobs[0].sourceId, "jNQXAC9IVRw");
    assert.equal(jobs[0].profileId, "original");
    console.log("F12 T10b: closed-app deep link created one persistent job");
  } finally {
    if (child.pid && child.exitCode === null) {
      const exited = new Promise((resolve) => child.once("exit", resolve));
      if (process.platform === "win32") {
        const killed = spawnSync("taskkill", ["/PID", String(child.pid), "/T", "/F"], {
          windowsHide: true,
          stdio: "ignore",
        });
        assert.equal(killed.status, 0, "could not terminate the test-owned app tree");
      } else child.kill("SIGTERM");
      await exited;
    }
    rmSync(scratch, { recursive: true, force: true });
  }
}
