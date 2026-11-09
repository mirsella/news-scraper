import { PostHog } from "posthog-node";

const key = process.env.POSTHOG_PROJECT_TOKEN;
const host = (process.env.POSTHOG_HOST || "https://eu.i.posthog.com").replace(/\/$/, "");
const service = "article-parser";
const environment = process.env.POSTHOG_ENVIRONMENT || "production";
if (key && !key.startsWith("phc_")) throw new Error("POSTHOG_PROJECT_TOKEN must be a public project token");
if (!host.startsWith("https://")) throw new Error("POSTHOG_HOST must use HTTPS");
const client = key ? new PostHog(key, { host, flushAt: 20, flushInterval: 10000, enableExceptionAutocapture: false }) : undefined;
if (!client) console.warn("PostHog disabled: POSTHOG_PROJECT_TOKEN is not configured");
const pending = new Set<Promise<void>>();

export function capture(event: string, properties: Record<string, unknown> = {}) {
  client?.capture({ distinctId: `service:${service}`, event, properties: {
    ...properties, service, environment, $process_person_profile: false,
  } });
}

export function log(body: string, level = "INFO", attributes: Record<string, string> = {}) {
  if (!key) return;
  if (pending.size >= 100) { console.warn("PostHog log queue full: dropping log"); return; }
  const delivery = (async () => {
    try {
      const response = await fetch(`${host}/i/v1/logs`, { method: "POST", signal: AbortSignal.timeout(5000),
        headers: { "Content-Type": "application/json", Authorization: `Bearer ${key}` },
        body: JSON.stringify({ resourceLogs: [{ resource: { attributes: [
          { key: "service.name", value: { stringValue: service } },
          { key: "deployment.environment", value: { stringValue: environment } },
        ] }, scopeLogs: [{ scope: { name: service }, logRecords: [{ timeUnixNano: `${Date.now()}000000`,
          severityText: level, severityNumber: level === "ERROR" ? 17 : 9,
          body: { stringValue: body }, attributes: Object.entries(attributes).map(([key, value]) => ({ key, value: { stringValue: value } })),
        }] }] }] }) });
      if (!response.ok) console.warn(`PostHog log delivery failed: HTTP ${response.status}`);
    } catch { console.warn("PostHog log delivery failed (network/timeout)"); }
  })();
  pending.add(delivery);
  void delivery.finally(() => pending.delete(delivery));
}

export function exception(error: unknown, operation: string) {
  const error_type = error instanceof Error ? error.name : typeof error;
  client?.captureException(new Error(`${operation} failed (${error_type})`), `service:${service}`,
    { service, environment, operation, $process_person_profile: false });
  log(`${operation} failed`, "ERROR", { error_type });
}

capture("service_started");
log("Service started");
const heartbeat = setInterval(() => capture("service_heartbeat"), 300000);
heartbeat.unref();
let stopping = false;
for (const signal of ["SIGTERM", "SIGINT"] as const) process.once(signal, async () => {
  if (stopping) return;
  stopping = true;
  clearInterval(heartbeat);
  capture("service_stopped");
  await Promise.allSettled([...pending]);
  await client?.shutdown();
  process.exit(0);
});
