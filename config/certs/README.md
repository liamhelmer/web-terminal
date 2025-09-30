# TLS Certificates for Web-Terminal

This directory contains TLS certificates for enabling HTTPS on the web-terminal server.

## ⚠️ IMPORTANT: Development vs Production

### Development Certificates

For **development only**, you can generate self-signed certificates:

```bash
# Generate self-signed certificate (valid for 365 days)
openssl req -x509 -newkey rsa:4096 -nodes \
  -keyout key.pem \
  -out cert.pem \
  -days 365 \
  -subj "/C=US/ST=State/L=City/O=Development/CN=localhost"

# Or with interactive prompts
openssl req -x509 -newkey rsa:4096 -nodes \
  -keyout key.pem \
  -out cert.pem \
  -days 365
```

**Browser Warning:** Self-signed certificates will show a security warning in browsers. This is expected for development certificates.

### Production Certificates

For **production**, use certificates from a trusted Certificate Authority (CA):

#### Option 1: Let's Encrypt (Recommended - Free)

```bash
# Install certbot
sudo apt-get install certbot  # Ubuntu/Debian
brew install certbot          # macOS

# Generate certificate (HTTP-01 challenge)
sudo certbot certonly --standalone -d your-domain.com

# Certificates will be in:
# Certificate: /etc/letsencrypt/live/your-domain.com/fullchain.pem
# Private Key: /etc/letsencrypt/live/your-domain.com/privkey.pem
```

#### Option 2: Commercial CA

Purchase a certificate from a commercial CA (DigiCert, GlobalSign, etc.) and follow their instructions.

#### Option 3: Internal CA

If using an internal Certificate Authority:

```bash
# Generate CSR (Certificate Signing Request)
openssl req -new -newkey rsa:4096 -nodes \
  -keyout key.pem \
  -out request.csr \
  -subj "/C=US/ST=State/L=City/O=YourOrg/CN=your-domain.com"

# Submit request.csr to your internal CA
# Receive signed certificate as cert.pem
```

## Configuration

### File Permissions (CRITICAL)

```bash
# Set correct permissions
chmod 600 key.pem    # Private key - read/write by owner only
chmod 644 cert.pem   # Certificate - readable by all
```

### Config File Setup

Edit `config/config.toml`:

```toml
[server]
host = "0.0.0.0"
port = 8080

[server.tls]
cert_path = "config/certs/cert.pem"
key_path = "config/certs/key.pem"
enforce_https = true  # Redirect HTTP to HTTPS
```

### Environment Variables (Alternative)

```bash
export TLS_CERT_PATH="/path/to/cert.pem"
export TLS_KEY_PATH="/path/to/key.pem"
export TLS_ENFORCE_HTTPS="true"
```

## Testing TLS Configuration

### 1. Verify Certificate

```bash
# Check certificate details
openssl x509 -in cert.pem -text -noout

# Check certificate expiration
openssl x509 -in cert.pem -noout -enddate

# Verify private key matches certificate
openssl x509 -noout -modulus -in cert.pem | openssl md5
openssl rsa -noout -modulus -in key.pem | openssl md5
# Both MD5 hashes should match
```

### 2. Test HTTPS Connection

```bash
# Start server with TLS
cargo run --release --features tls

# Test HTTPS endpoint
curl -k https://localhost:8080/api/v1/health

# Test with certificate validation (production)
curl --cacert cert.pem https://localhost:8080/api/v1/health

# Test WebSocket over TLS
websocat wss://localhost:8080/ws
```

### 3. Browser Testing

1. Open browser to `https://localhost:8080`
2. For self-signed certs, accept the security warning
3. Verify green lock icon in address bar (production certs)
4. Check certificate details in browser

## TLS Configuration Options

### Cipher Suites

The server uses secure cipher suites by default (rustls):

- TLS 1.2 and TLS 1.3 only
- Forward secrecy (ECDHE/DHE key exchange)
- AEAD ciphers (AES-GCM, ChaCha20-Poly1305)
- No weak ciphers (RC4, 3DES, MD5)

### HSTS (HTTP Strict Transport Security)

Enabled by default with 1-year max-age:

```
Strict-Transport-Security: max-age=31536000
```

This tells browsers to ALWAYS use HTTPS for future connections.

### Certificate Rotation

#### Automated Rotation (Let's Encrypt)

```bash
# Renew certificates (runs automatically with certbot)
sudo certbot renew

# Restart web-terminal after renewal
sudo systemctl restart web-terminal
```

#### Manual Rotation

1. Generate/obtain new certificate
2. Replace `cert.pem` and `key.pem`
3. Restart server (no downtime with graceful reload)

```bash
# Graceful reload (send SIGHUP)
kill -HUP $(pgrep web-terminal)
```

## Security Best Practices

### ✅ DO

- Use certificates from trusted CAs in production
- Set restrictive file permissions (600 for private key)
- Enable `enforce_https` to redirect HTTP to HTTPS
- Monitor certificate expiration dates
- Automate certificate renewal (Let's Encrypt)
- Use strong key sizes (4096-bit RSA or 256-bit ECC)
- Test certificate configuration before deployment

### ❌ DON'T

- Commit private keys to version control (`.gitignore` already excludes `*.pem`)
- Use self-signed certificates in production
- Use weak key sizes (< 2048-bit RSA)
- Share private keys between environments
- Ignore certificate expiration warnings
- Use deprecated TLS versions (1.0, 1.1)

## Troubleshooting

### Certificate Not Found

```
Error: Failed to open certificate file: config/certs/cert.pem
```

**Solution:** Verify the file paths in your configuration match the actual certificate locations.

### Permission Denied

```
Error: Cannot read private key file: Permission denied
```

**Solution:** Check file permissions and ownership.

```bash
# Fix ownership (replace `user` with your username)
sudo chown user:user cert.pem key.pem

# Fix permissions
chmod 600 key.pem
chmod 644 cert.pem
```

### Certificate Expired

```
Error: certificate has expired
```

**Solution:** Generate/obtain a new certificate and replace the old one.

### TLS Handshake Failed

```
Error: error:1408F10B:SSL routines:ssl3_get_record:wrong version number
```

**Solution:** Client may be using an unsupported TLS version. Server requires TLS 1.2 or higher.

### Browser Security Warning

**Self-signed certificate:** Expected for development. Accept the warning or install the certificate in your browser's trust store.

**Production certificate:** Verify the certificate is correctly installed and matches your domain name.

## References

- [Let's Encrypt](https://letsencrypt.org/)
- [Rustls Documentation](https://docs.rs/rustls/)
- [Mozilla SSL Configuration Generator](https://ssl-config.mozilla.org/)
- [SSL Labs Server Test](https://www.ssllabs.com/ssltest/)

---

**Last Updated:** 2025-09-29
**Version:** 1.0.0