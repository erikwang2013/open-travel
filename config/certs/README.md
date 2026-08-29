# TLS 证书放置说明

nginx 容器内证书路径为 `/etc/nginx/certs/`（由 docker-compose 挂载本目录，
见 `config/docker-compose.yml` nginx 服务 volumes）。nginx.conf 的 443
监听块引用 `fullchain.pem` 与 `privkey.pem`。

## 当前状态

本目录中的 `fullchain.pem` / `privkey.pem` 是**开发占位自签证书**（CN=
open-travel-placeholder），仅保证本地 443 可启动、配置可解析。
浏览器访问会提示不受信任，**禁止用于生产**。

## 上线前替换为真实证书

方式一（推荐，Let's Encrypt 自动续期）：

```bash
# 在域名指向的服务器上执行；certbot 生成的证书复制到本目录
certbot certonly --nginx -d api.example.com -d www.example.com
cp /etc/letsencrypt/live/api.example.com/fullchain.pem config/certs/
cp /etc/letsencrypt/live/api.example.com/privkey.pem config/certs/
docker compose -f config/docker-compose.yml exec nginx nginx -s reload
```

方式二（企业/商业证书）：将 CA 颁发的链与私钥分别命名为
`fullchain.pem` / `privkey.pem` 放入本目录后 reload。

## 注意事项

- 私钥 `privkey.pem` 是敏感文件：替换为真实证书后**不要提交到 git**
  （.env 同理），并 `chmod 600`。
- 证书过期前需续期（HSTS max-age=31536000 期间浏览器强制 HTTPS）。
- 替换后执行 `scripts/health_check.sh` 确认服务正常。
