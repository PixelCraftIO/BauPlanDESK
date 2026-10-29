export async function register() {
  if (process.env.NEXT_RUNTIME === "nodejs") {
    const { einrichten } = await import("./lib/einrichtung");
    await einrichten();
  }
}
