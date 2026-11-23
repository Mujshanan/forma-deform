const server = Bun.serve({
  port: 3000,
  async fetch(request) {
    const path = new URL(request.url).pathname;

    if (path === "/" || path === "/index.html") {
      return new Response(Bun.file("./index.html"), {
        headers: { "content-type": "text/html; charset=utf-8" }
      });
    }

    if (path === "/studio.css") {
      return new Response(Bun.file("./studio.css"), {
        headers: { "content-type": "text/css; charset=utf-8" }
      });
    }

    if (path === "/main.js") {
      const result = await Bun.build({
        entrypoints: ["./src/studio/main.ts"],
        target: "browser"
      });
      if (!result.success) {
        return new Response("Could not build the Studio entry point.", { status: 500 });
      }
      const output = result.outputs.find(file => file.path.endsWith(".js"));
      if (!output) return new Response("Studio bundle was not produced.", { status: 500 });
      return new Response(await output.text(), {
        headers: { "content-type": "text/javascript; charset=utf-8" }
      });
    }

    return new Response("Not found", { status: 404 });
  }
});

console.log(`Forma Deform Studio running at http://localhost:${server.port}`);
