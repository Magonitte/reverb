// F12 T10: a second invocation hands its link to the running application.
import { spawn, spawnSync } from "node:child_process";
import { basename, dirname } from "node:path";
import { invoke, waitForHome } from "../helpers.mjs";

describe("F12 deep links in the real app", () => {
  it("T11: autostart registers the test executable with --minimized and can be disabled", async () => {
    await waitForHome();
    const name = `Reverb-test-${basename(dirname(process.env.REVERB_DATA_DIR)).replace(/[^a-z0-9]/gi, "")}`;
    const read = () => {
      const result = spawnSync(
        "powershell.exe",
        [
          "-NoProfile",
          "-Command",
          "$taskValue = Get-ItemPropertyValue -LiteralPath 'HKCU:\\Software\\Microsoft\\Windows\\CurrentVersion\\Run' -Name $env:REVERB_TEST_AUTOSTART_NAME -ErrorAction SilentlyContinue; ConvertTo-Json -InputObject $taskValue",
        ],
        {
          encoding: "utf8",
          windowsHide: true,
          env: { ...process.env, REVERB_TEST_AUTOSTART_NAME: name },
        },
      );
      expect(result.status).toBe(0);
      return JSON.parse(result.stdout);
    };
    expect(read()).toBeNull();
    try {
      await invoke("settings_update", { patch: { launchAtStartup: true } });
      const command = read();
      expect(command).toContain(process.env.REVERB_E2E_APP);
      expect(command).toContain("--minimized");
    } finally {
      await invoke("settings_update", { patch: { launchAtStartup: false } });
    }
    expect(read()).toBeNull();
  });
  it("forwards a link from a second instance and creates exactly one job", async () => {
    await waitForHome();
    await invoke("queue_pause");
    const before = await invoke("jobs_list");
    const child = spawn(
      process.env.REVERB_E2E_APP,
      ["reverb://add?url=https%3A%2F%2Fwww.youtube.com%2Fwatch%3Fv%3DjNQXAC9IVRw&profile=mp3_v0"],
      { windowsHide: true, stdio: "ignore" },
    );
    await new Promise((resolve, reject) => {
      const timer = setTimeout(() => {
        child.kill();
        reject(new Error("second instance stayed alive"));
      }, 30000);
      child.once("error", (error) => {
        clearTimeout(timer);
        reject(error);
      });
      child.once("exit", (code) => {
        clearTimeout(timer);
        if (code === 0) resolve();
        else reject(new Error(`second instance exited ${code}`));
      });
    });
    await browser.waitUntil(async () => (await invoke("jobs_list")).length === before.length + 1, {
      timeout: 30000,
      timeoutMsg: "forwarded deep link did not create a job",
    });
    const jobs = await invoke("jobs_list");
    const added = jobs.filter((job) => !before.some((previous) => previous.id === job.id));
    expect(added).toHaveLength(1);
    expect(added[0].sourceId).toBe("jNQXAC9IVRw");
    expect(added[0].profileId).toBe("mp3_v0");
    await invoke("job_cancel", { id: added[0].id });
    await invoke("queue_resume");
  });
});
