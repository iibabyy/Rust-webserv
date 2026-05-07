# Rust Web Server
A high-performance web server implemented in Rust, inspired by nginx's architecture and features.

## Quick Start

### Prerequisites
- Cargo
- Optional (for CGI): Python, PHP, Bash (Update the binaries paths in the config file if needed)

### Run
```bash
unzip URIs
cargo run
```

## Performances

This test is not very scientific and was simply a small experiment to see roughly how my web server is performing

```bash
# Ran on a 16Go RAM, M4 MacBook
$ bombardier http://localhost:8000/tests/simple.txt --latencies --fasthttp -H "Connection: Close" -c 1000
Bombarding http://localhost:8000/tests/simple.txt for 10s using 1000 connection(s)
[============================================================================] 10s
Done!
Statistics        Avg      Stdev        Max
  Reqs/sec     45744.95    7174.15   84265.67
  Latency       21.89ms     7.81ms   170.64ms
  Latency Distribution
     50%    19.44ms
     75%    28.02ms
     90%    37.85ms
     95%    44.99ms
     99%    60.42ms
  HTTP codes:
    1xx - 0, 2xx - 457115, 3xx - 0, 4xx - 0, 5xx - 0
    others - 0
  Throughput:     7.66MB/s
```

## Features

### Core Functionality
- HTTP/1.1 protocol support
- Keep-alive connection handling
- Asynchronous I/O using Tokio
- Configurable server blocks
- Multiple server support (virtual hosting)

### Request Handling
- Support for GET and POST methods
- Content-Length validation
- Maximum body size limits
- Custom error pages and redirects

### Static File Serving
- Directory listing with auto-indexing
- Custom index file configuration
- Root directory configuration
- Stylish auto-generated directory listings with gradient backgrounds

### Configuration
- Server name configuration
- Port binding configuration
- Upload folder specification
- Client max body size limits
- Multiple methods restriction
- Custom error pages
- Return directives for redirects

### Location Blocks
- Path-based configuration blocks
- Alias support
- Internal location handling
- Custom routing rules

### CGI Support
- Dynamic content generation through CGI scripts
- CGI environment variable handling
- Request body forwarding to CGI scripts
- Configurable CGI extensions and paths

## Configuration Example
```nginx
server {
    listen 8080;
    server_name example.com;
    root /var/www/html;
    
    client_max_body_size 10M;
    auto_index on;
    
    location /uploads {
        upload_folder /var/www/uploads;
        methods GET POST;
    }
    
    location /cgi-bin {
        cgi .php /usr/bin/php-cgi;
    }
}
```
