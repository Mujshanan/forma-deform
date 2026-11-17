const server = Bun.serve({
  port: 3000,
  async fetch(request) {
    const url = new URL(request.url);
    const path = url.pathname === "/" ? "/index.html" : url.pathname;
    const file = Bun.file(`.${path}`);
    if (!(await file.exists())) return new Response("Not found", { status: 404 });
    const contentType = path.endsWith(".html") ? "text/html; charset=utf-8"
      : path.endsWith(".css") ? "text/css; charset=utf-8"
      : path.endsWith(".js") ? "text/javascript; charset=utf-8"
      : "application/octet-stream";
    return new Response(file, { headers: { "content-type": contentType } });
  }
});
console.log(`Forma Deform demo running at http://localhost:${server.port}`);
