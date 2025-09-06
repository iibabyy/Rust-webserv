# Rust Web Server
A high-performance web server implemented in Rust, inspired by nginx's architecture and features.

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

### CGI Support
- Dynamic content generation through CGI scripts
- CGI environment variable handling
- Request body forwarding to CGI scripts
- Configurable CGI extensions and paths

### Location Blocks
- Path-based configuration blocks
- Alias support
- Internal location handling
- Custom routing rules

### Configuration
- Server name configuration
- Port binding configuration
- Upload folder specification
- Client max body size limits
- Multiple methods restriction
- Custom error pages
- Return directives for redirects

### Error Handling
- Comprehensive error reporting
- Custom error page mapping
- Error redirects with status codes
- Graceful connection handling

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

## Building and Running

### Prerequisites
- Rust (latest stable version)
- Cargo

### Build
```bash
cargo build --release
```

### Run
```bash
cargo run --release
```

## Performances

This test is not very scientific and were simply small experiments to see roughly how my web server is perfoming

```bash
bombardier http://localhost:8080/ -c 1000 --latencies --fasthttp -H "Connection
: Close"
Bombarding http://localhost:8080/ for 10s using 1000 connection(s)
[========================================================================================================================] 10s
Done!
Statistics        Avg      Stdev        Max
  Reqs/sec     10256.26    2923.56   19914.07
  Latency       97.03ms    30.06ms   511.95ms
  Latency Distribution
     50%    93.78ms
     75%   108.64ms
     90%   124.37ms
     95%   136.59ms
     99%   181.70ms
  HTTP codes:
    1xx - 0, 2xx - 103328, 3xx - 0, 4xx - 0, 5xx - 0
    others - 0
```

## Contributing
Contributions are welcome! Please feel free to submit a Pull Request.
