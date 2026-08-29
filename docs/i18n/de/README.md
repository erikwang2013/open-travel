[简体中文](../../README.md) | [English](en/README.md) | [日本語](ja/README.md) | [한국어](ko/README.md) | [Русский](ru/README.md) | [Deutsch](README.md) | [Français](fr/README.md) | [Español](es/README.md) | [Português](pt/README.md) | [हिन्दी](hi/README.md) | [العربية](ar/README.md) | [বাংলা](bn/README.md) | [Bahasa Indonesia](id/README.md)

# Open Travel — Globale Reiseplattform

<p align="center"><img src="../../mascot.svg" alt="Travly 小旅 — Open Travel 吉祥物" width="180"></p>


> Eine Reisebuchungsplattform für Nutzer weltweit: Rust-Mikroservice-Backend + Flutter / HarmonyOS-Clienten mit Unterstützung für **12+ Sprachen**, internationale Zahlungen und mehrsprachige Suche.

## Projektübersicht

Open Travel ist ein Monorepo einer globalen Reiseplattform, aufgebaut auf **e-cat (eine Katze)** — einem Rust-Mikroservice-Framework auf Augenhöhe mit [go-kratos/kratos](https://github.com/go-kratos/kratos) v3 (v3.0.2 · 51 Crates) — für ein leistungsstarkes Backend, zusammen mit Flutter-Mehrplattform- und nativen HarmonyOS-Clienten, die weltweit ein einheitliches Reisebuchungserlebnis bieten.

| Dimension | Beschreibung |
| :--- | :--- |
| **Backend-Framework** | e-cat (Rust): HTTP/axum + gRPC/tonic, Mikroservice-Ökosystem aus 51 Crates |
| **Mehrplattform-Clienten** | `apps/client/flutter` (iOS / Android / Web / Desktop), `apps/client/harmonyos` (HarmonyOS) |
| **Datenbank** | MySQL (Datenbank `travel`, Tabellenpräfix `travel_`) + Redis-Cache + OpenSearch-Mehrsprachsuche |
| **Sicherheit** | ecat-security / ecat-auth (JWT) / ecat-tls: Authentifizierung, Audit, Rate-Limiting, Schutz vor Injektionen |
| **Internationalisierung** | 12+ Sprachen als ARB-Sprachpakete, RTL-Unterstützung, mehrsprachige OpenSearch-Tokenisierung |
| **Zahlungen** | WeChat Pay, Alipay |

## Kernfunktionen

- 🏨 Mehrsprachige Suche und Buchung von Reisezielen / Hotels / Flügen
- 🌍 Eigenständige Anpassung für 12+ Sprachen (Chinesisch, Englisch, Japanisch, Koreanisch, Arabisch, Spanisch, Französisch, Deutsch …)
- 💳 Internationale Zahlungen (WeChat Pay / Alipay)
- 🔐 Verteidigung in der Tiefe: TLS 1.3, JWT-Authentifizierung, Audit-Logs, Eingabefilterung, Rate-Limiting, HMAC-Verifizierung von Zahlungs-Callbacks, interne Service-Authentifizierung
- 📱 Konsistentes Erlebnis auf allen Plattformen: Flutter (iOS/Android/Web/Desktop) + HarmonyOS

## Architekturdiagramm

![Architekturdiagramm](../../svg/de/architecture.svg)

## Funktionsdiagramm

![Funktionsdiagramm](../../svg/de/features.svg)

## Projektdiagramm

![Projektdiagramm](../../svg/de/project.svg)

## Request-Zyklus-Diagramm

![Request-Zyklus-Diagramm](../../svg/de/request-cycle.svg)

## Sicherheitsarchitektur-Diagramm

![Sicherheitsarchitektur-Diagramm](../../svg/de/security-architecture.svg)

## Projektstruktur-Diagramm

![Projektstruktur-Diagramm](../../svg/de/project-structure.svg)

## Projektstruktur

```
open-travel/
├── apps/                  # Verzeichnis der Clienten
│   ├── flutter/           # Flutter: iOS / Android / Web / Desktop (i18n in 12+ Sprachen)
│   └── harmonyos/         # Nativer HarmonyOS-Client
├── e-cat/                 # Rust-Mikroservice-Framework e-cat (51 Crates)
├── docs/                  # Projektplanung, Diagramme (SVG), Zahlungs-QR-Codes
├── config/                # Umgebungs- und Deployment-Konfiguration
└── README.md
```

## Datenbank

- Datenbankname: `travel`
- Tabellenpräfix: `travel_` (z. B. `travel_users`, `travel_orders`, `travel_reviews`)
- Zusätzliche Speicher: Redis (Sitzungen / Beliebtheits-Cache), OpenSearch (mehrsprachige Suchindizes)

> Detaillierte technische Planung: [docs/travel-project-planning.md](../../travel-project-planning.md).

## Schnellstart

```bash
cd e-cat
cargo check -p user-service -p booking-service -p admin-service   # Kompilierprüfung der Geschäftsdienste
```

| Dienst | Port | Beschreibung |
|---|---|---|
| user-service | 8001 | Benutzerregistrierung / Anmeldung / Profil |
| booking-service | 8002 | Datumsangaben beliebter Reiseziele + Sehenswürdigkeiten-Liste / -Detail |
| admin-service | 8003 | Admin: Anmeldung + CRUD für Reiseziele / Sehenswürdigkeiten |
| Nginx-Gateway | 8082→80 | Routing nach Präfixen `/api/user/`, `/api/booking/`, `/api/admin/` |
| MySQL | 3308→3306 | Datenquelle |
| Redis | 6381→6379 | Cache / Rate-Limiting |
| OpenSearch | 9201→9200 | Mehrsprachige Suche |

Die Admin-Flutter-Web-App liegt unter `apps/admin/`; das Standard-Admin-Konto für die Entwicklung ist `admin@travel.local` / `Admin@123` (nur lokal).

---

## Unterstützen Sie uns

Wenn Ihnen dieses Projekt geholfen hat, laden Sie den Autor gern auf einen Kaffee ein ☕

<p align="center">
  <strong>WeChat Pay</strong> &nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp; <strong>Alipay</strong><br/>
  <img src="../../weixinpay.png" alt="WeChat-Pay-QR-Code" width="130" height="130" />
  &nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;
  <img src="../../alipay.png" alt="Alipay-QR-Code" width="130" height="130" />
</p>

### Globale Überweisung (Global Bank Transfer)

**Empfängerinformationen**

- Empfängername: WANG KEXUN
- Empfängerkontonummer: 881015918251

**Empfängerbank**

- SWIFT-Code ZA Bank: AABLHKHHXXX
- Bankname: ZA Bank Limited
- Bankleitzahl: 387
- Bankadresse: Core F, Cyberport 3, 100 Cyberport Road, Hong Kong

**Korrespondenzbank für internationale Überweisungen (falls erforderlich)**

Bitte beachten Sie: Dies sind die Angaben der Korrespondenzbank (Zwischenbank) für internationale Überweisungen, nicht der Empfängerbank. Fragen Sie bei Ihrer überweisenden Bank nach, ob Angaben zur Korrespondenzbank erforderlich sind.

Für Überweisungen in Hongkong-Dollar, Renminbi und US-Dollar ist die Korrespondenzbank **Citibank** —

- Bankname: Citibank N.A. Hong Kong
- SWIFT-Code: CITIHKHXXXX
- Bankleitzahl: 006
- Filialname: Hong Kong Branch
- Filialnummer: 391
- Bankadresse: Citibank Tower, Citibank Plaza, 3 Garden Road, Central, Hong Kong

Für Überweisungen in anderen Währungen ist die Korrespondenzbank **BNY Mellon** —

- Bankname: THE BANK OF NEW YORK MELLON
- SWIFT-Code: IRVTUS3NXXX
- Bankadresse: THE BANK OF NEW YORK MELLON, 240 GREENWICH STREET, NEW YORK, United States

### Krypto-Spenden (Crypto Donation)

Wenn dieses Projekt Ihnen hilft, scannen Sie gerne den QR-Code, um zu spenden. Vielen Dank!

| <img src="../../coin/1.jpg" width="200" alt="BNB Smart Chain (BEP20)"><br>**BNB Smart Chain (BEP20)**<br>`0x355d429f97511897ccb4e271ec888205f9ab6629` | <img src="../../coin/2.jpg" width="200" alt="Tron (TRC20)"><br>**Tron (TRC20)**<br>`TEdDHWLajt1XvqtPDWmQctdrJaC3pzZZzz` |
| <img src="../../coin/3.jpg" width="200" alt="Ethereum (ERC20)"><br>**Ethereum (ERC20)**<br>`0x355d429f97511897ccb4e271ec888205f9ab6629` | <img src="../../coin/4.jpg" width="200" alt="Aptos"><br>**Aptos**<br>`0x836e3780edfc3f7b2372b39e2a1a3a5d7adfaccd96c726f21cfde1b50dd68030` |
| <img src="../../coin/5.jpg" width="200" alt="Plasma"><br>**Plasma**<br>`0x355d429f97511897ccb4e271ec888205f9ab6629` | <img src="../../coin/6.jpg" width="200" alt="Polygon POS"><br>**Polygon POS**<br>`0x355d429f97511897ccb4e271ec888205f9ab6629` |
| <img src="../../coin/7.jpg" width="200" alt="Solana"><br>**Solana**<br>`2hfhboHdmdrYsY25XfQSsEWxq5ip4EQsR7f4AzSRMUyr` | <img src="../../coin/8.jpg" width="200" alt="The Open Network (TON)"><br>**The Open Network (TON)**<br>`UQB9kFQohzmXUir9QSSZq01iwl9aQZIDdBpNmDklljRtCoGK` |
| <img src="../../coin/9.jpg" width="200" alt="Arbitrum One"><br>**Arbitrum One**<br>`0x355d429f97511897ccb4e271ec888205f9ab6629` | <img src="../../coin/10.jpg" width="200" alt="AVAX C-Chain"><br>**AVAX C-Chain**<br>`0x355d429f97511897ccb4e271ec888205f9ab6629` |

