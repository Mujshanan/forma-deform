const server = Bun.serve({
  port: 3000,
  async fetch(request) {
    const path = new URL(request.url).pathname;
    if (path === "/" || path === "/index.html") {
      return new Response(Bun.file("./index.html"), { headers: { "content-type": "text/html; charset=utf-8" } });
    }
    if (path === "/style.css") {
      return new Response(Bun.file("./style.css"), { headers: { "content-type": "text/css; charset=utf-8" } });
    }
    if (path === "/main.js") {
      const result = await Bun.build({ entrypoints: ["./src/demo/main.ts"], target: "browser", write: false });
      if (!result.success) return new Response("Could not build demo entry point.", { status: 500 });
      const output = result.outputs.find(file => file.path.endsWith(".js"));
      if (!output) return new Response("Demo bundle was not produced.", { status: 500 });
      return new Response(await output.text(), { headers: { "content-type": "text/javascript; charset=utf-8" } });
    }
    return new Response("Not found", { status: 404 });
  }
});
console.log(`Forma Deform demo running at http://localhost:${server.port}`);
