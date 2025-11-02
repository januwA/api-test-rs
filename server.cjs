const http = require("http");
const url = require("url");
const fs = require("fs");
const path = require("path");

const PORT = 3000;
const UPLOAD_DIR = path.join(__dirname, "target");

// 确保上传目录存在
if (!fs.existsSync(UPLOAD_DIR)) {
  fs.mkdirSync(UPLOAD_DIR, { recursive: true });
}

const server = http.createServer((req, res) => {
  const parsedUrl = url.parse(req.url, true);
  const pathname = parsedUrl.pathname;
  const method = req.method;

  res.setHeader("Content-Type", "application/json");

  if (pathname === "/ping" && method === "GET") {
    console.log(req.url);
    
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
  } else if (pathname === "/upload" && method === "POST") {
    const contentType = req.headers["content-type"] || "";

    if (contentType.includes("multipart/form-data")) {
      const boundary = contentType.split("boundary=")[1];
      let body = [];

      req.on("data", (chunk) => {
        body.push(chunk);
      });

      req.on("end", () => {
        const buffer = Buffer.concat(body);
        const boundaryBuffer = Buffer.from(`--${boundary}`);
        const files = [];
        const fields = {};

        // 分割 multipart 数据
        let start = 0;
        while (true) {
          const boundaryIndex = buffer.indexOf(boundaryBuffer, start);
          if (boundaryIndex === -1) break;

          const nextBoundaryIndex = buffer.indexOf(boundaryBuffer, boundaryIndex + boundaryBuffer.length);
          if (nextBoundaryIndex === -1) break;

          const part = buffer.slice(boundaryIndex + boundaryBuffer.length, nextBoundaryIndex);

          // 查找头部结束位置
          const headerEnd = part.indexOf(Buffer.from("\r\n\r\n"));
          if (headerEnd === -1) {
            start = nextBoundaryIndex;
            continue;
          }

          const headerPart = part.slice(0, headerEnd).toString();
          const dataPart = part.slice(headerEnd + 4, part.length - 2); // 去掉末尾的 \r\n

          // 解析 Content-Disposition
          const nameMatch = headerPart.match(/name="([^"]+)"/);
          const filenameMatch = headerPart.match(/filename="([^"]+)"/);

          if (!nameMatch) {
            start = nextBoundaryIndex;
            continue;
          }

          const fieldName = nameMatch[1];
          const filename = filenameMatch ? filenameMatch[1] : null;

          if (filename) {
            // 这是文件
            const contentTypeMatch = headerPart.match(/Content-Type: (.+)/);
            const fileContentType = contentTypeMatch ? contentTypeMatch[1].trim() : "application/octet-stream";

            // 保存文件到 target 目录
            const timestamp = Date.now();
            // 提取文件扩展名
            const extname = path.extname(filename); // 例如: ".jpg", ".png"
            const basename = path.basename(filename, extname); // 不含扩展名的文件名
            const safeBasename = basename.replace(/[^a-zA-Z0-9._-]/g, "_");
            const savedFilename = `${timestamp}_${safeBasename}${extname}`;
            const filePath = path.join(UPLOAD_DIR, savedFilename);

            try {
              fs.writeFileSync(filePath, dataPart);

              files.push({
                fieldName: fieldName,
                originalFilename: filename,
                savedFilename: savedFilename,
                savedPath: filePath,
                contentType: fileContentType,
                size: dataPart.length,
              });
            } catch (err) {
              console.error("Error saving file:", err);
              files.push({
                fieldName: fieldName,
                originalFilename: filename,
                error: err.message,
              });
            }
          } else {
            // 这是普通字段
            fields[fieldName] = dataPart.toString();
          }

          start = nextBoundaryIndex;
        }

        res.writeHead(200);
        res.end(
          JSON.stringify({
            success: true,
            message: "File upload processed and saved",
            uploadDir: UPLOAD_DIR,
            files: files,
            fields: fields,
            totalFiles: files.length,
            timestamp: Date.now(),
          })
        );
      });
    } else {
      res.writeHead(400);
      res.end(
        JSON.stringify({
          error: "Expected multipart/form-data",
          received: contentType,
        })
      );
    }
  } else {
    res.writeHead(404);
    res.end(JSON.stringify({ error: "Not Found" }));
  }
});

server.listen(PORT, "0.0.0.0", () => {
  console.log(`Test server running at http://127.0.0.1:${PORT}/`);
  console.log(`Upload directory: ${UPLOAD_DIR}`);
  console.log(`
Available endpoints:
  GET  /ping              - Simple ping response
  POST /echo              - Echo back POST body
  GET  /delay?ms=1000     - Delayed response (default 1000ms)
  GET  /status?code=200   - Return specific status code
  GET  /user              - Get user info
  GET  /error             - Return 500 error
  GET  /random            - 30% chance to fail (500), 70% success (200)
  POST /upload            - Upload files (multipart/form-data, saved to target/)
  `);
});
