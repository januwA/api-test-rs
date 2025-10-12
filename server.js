const http = require("http");
const url = require("url");

const PORT = 3000;

const server = http.createServer((req, res) => {
  const parsedUrl = url.parse(req.url, true);
  const pathname = parsedUrl.pathname;
  const method = req.method;

  res.setHeader("Content-Type", "application/json");

  if (pathname === "/ping" && method === "GET") {
    res.writeHead(200);
    res.end(JSON.stringify({ message: "pong", timestamp: Date.now() }));
  } else if (pathname === "/echo" && method === "POST") {
    let body = "";
    req.on("data", (chunk) => {
      body += chunk.toString();
    });
    req.on("end", () => {
      res.writeHead(200);
      res.end(
        JSON.stringify({
          body,
          headers: req.headers,
          query: parsedUrl.query,
          timestamp: Date.now(),
        })
      );
    });
  } else if (pathname === "/delay" && method === "GET") {
    const delay = parseInt(parsedUrl.query.ms) || 1000;
    setTimeout(() => {
      res.writeHead(200);
      res.end(
        JSON.stringify({
          delayed: delay,
          timestamp: Date.now(),
        })
      );
    }, delay);
  } else if (pathname === "/status" && method === "GET") {
    const code = parseInt(parsedUrl.query.code) || 200;
    res.writeHead(code);
    res.end(
      JSON.stringify({
        status: code,
        message: http.STATUS_CODES[code],
      })
    );
  } else if (pathname === "/user" && method === "GET") {
    res.writeHead(200);
    res.end(
      JSON.stringify({
        id: 1,
        name: "Test User",
        email: "test@example.com",
      })
    );
  } else if (pathname === "/error" && method === "GET") {
    res.writeHead(500);
    res.end(JSON.stringify({ error: "Internal Server Error" }));
  } else if (pathname === "/random" && method === "GET") {
    const randomValue = Math.random();
    if (randomValue < 0.3) {
      res.writeHead(500);
      res.end(
        JSON.stringify({
          success: false,
          error: "Random failure occurred",
          probability: "30%",
          timestamp: Date.now(),
        })
      );
    } else {
      res.writeHead(200);
      res.end(
        JSON.stringify({
          success: true,
          data: "Request succeeded",
          probability: "70%",
          timestamp: Date.now(),
        })
      );
    }
  } else {
    res.writeHead(404);
    res.end(JSON.stringify({ error: "Not Found" }));
  }
});

server.listen(PORT, "127.0.0.1", () => {
  console.log(`Test server running at http://127.0.0.1:${PORT}/`);
  console.log(`
Available endpoints:
  GET  /ping              - Simple ping response
  POST /echo              - Echo back POST body
  GET  /delay?ms=1000     - Delayed response (default 1000ms)
  GET  /status?code=200   - Return specific status code
  GET  /user              - Get user info
  GET  /error             - Return 500 error
  GET  /random            - 30% chance to fail (500), 70% success (200)
  `);
});
