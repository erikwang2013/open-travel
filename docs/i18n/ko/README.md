[简体中文](../../README.md) | [English](../en/README.md) | [日本語](../ja/README.md) | [한국어](README.md) | [Русский](../ru/README.md) | [Deutsch](../de/README.md) | [Français](../fr/README.md) | [Español](../es/README.md) | [Português](../pt/README.md) | [हिन्दी](../hi/README.md) | [العربية](../ar/README.md) | [বাংলা](../bn/README.md) | [Bahasa Indonesia](../id/README.md)

# Open Travel — 글로벌 여행 플랫폼

<p align="center"><img src="../../mascot.svg" alt="Travly 小旅 — Open Travel 吉祥物" width="180"></p>


> 전 세계 사용자를 위한 여행 예약 플랫폼: Rust 마이크로서비스 백엔드 + Flutter / HarmonyOS 멀티 플랫폼 클라이언트, **12+ 언어** 지원, 국제 결제 및 다국어 검색.

## 프로젝트 소개

Open Travel은 [go-kratos/kratos](https://github.com/go-kratos/kratos) v3를 벤치마킹한 **Rust 마이크로서비스 프레임워크**(v3.0.3 · 51 crates)인 **e-cat(고양이 한 마리)** 을 사용해 고성능 백엔드를 구축하고, Flutter 멀티 플랫폼 및 HarmonyOS 네이티브 클라이언트를 결합하여 전 세계 사용자에게 통일된 여행 예약 경험을 제공하는 글로벌 여행 플랫폼 monorepo입니다.

| 항목 | 설명 |
| :--- | :--- |
| **백엔드 프레임워크** | e-cat(Rust): HTTP/axum + gRPC/tonic, 51 crates 마이크로서비스 생태계 |
| **멀티 플랫폼 클라이언트** | `apps/client/flutter`(iOS / Android / Web / Desktop), `apps/client/harmonyos`(HarmonyOS) |
| **데이터베이스** | MySQL(DB명 `travel`, 테이블 접두사 `travel_`) + Redis 캐시 + OpenSearch 다국어 검색 |
| **보안** | ecat-security / ecat-auth(JWT) / ecat-tls: 인증, 감사, 속도 제한, 주입 방지 |
| **국제화** | 12+ 언어 ARB 로케일 팩, RTL 지원, OpenSearch 다국어 토큰화 |
| **결제** | WeChat Pay, Alipay |

## 핵심 기능

- 🏨 목적지 / 호텔 / 항공권 다국어 검색 및 예약
- 🌍 12+ 언어 개별 지원(중국어, 영어, 일본어, 한국어, 아랍어, 스페인어, 프랑스어, 독일어…)
- 💳 국제 결제(WeChat Pay / Alipay)
- 🔐 심층 방어: TLS 1.3, JWT 인증, 감사 로그, 입력 필터링, 속도 제한, 결제 콜백 HMAC 검증, 내부 서비스 인증
- 📱 멀티 플랫폼 일관된 경험: Flutter(iOS/Android/Web/Desktop) + HarmonyOS

## 아키텍처 다이어그램

![아키텍처 다이어그램](../../svg/ko/architecture.svg)

## 기능 다이어그램

![기능 다이어그램](../../svg/ko/features.svg)

## 프로젝트 다이어그램

![프로젝트 다이어그램](../../svg/ko/project.svg)

## 요청 사이클 다이어그램

![요청 사이클 다이어그램](../../svg/ko/request-cycle.svg)

## 보안 아키텍처 다이어그램

![보안 아키텍처 다이어그램](../../svg/ko/security-architecture.svg)

## 프로젝트 구조

```
open-travel/
├── apps/                  # 멀티 플랫폼 클라이언트 디렉터리
│   ├── flutter/           # Flutter: iOS / Android / Web / Desktop(12+ 언어 i18n)
│   └── harmonyos/         # HarmonyOS 네이티브 클라이언트
├── e-cat/                 # e-cat 프레임워크 + 비즈니스 서비스(단일 Cargo workspace)
│   ├── ecat*/             # 51개 ecat-* 프레임워크 crate
│   ├── ecat/              # 메인 프레임워크 crate: 파사드 + 비즈니스 모듈 (src/business/) + 서비스 엔트리 (src/bin/)
│   ├── config/            # 프레임워크 설정 예제
│   └── examples/          # 프레임워크 예제 프로젝트
├── docs/                  # 프로젝트 계획, 아키텍처 다이어그램(SVG), 결제 QR 코드
├── config/                # 환경 및 배포 구성
└── README.md
```

## 데이터베이스

- 데이터베이스 이름: `travel`
- 테이블 접두사: `travel_`(예: `travel_users`, `travel_orders`, `travel_reviews`)
- 보조 스토리지: Redis(세션 / 인기 캐시), OpenSearch(다국어 검색 인덱스)

> 자세한 기술 계획은 [docs/travel-project-planning.md](../../travel-project-planning.md)를 참조하세요.

## 빠른 시작

```bash
cd e-cat
cargo check -p ecat --bins   # 비즈니스 서비스 컴파일 확인
```

| 서비스 | 포트 | 설명 |
|---|---|---|
| user-service | 8001 | 사용자 가입 / 로그인 / 프로필 |
| booking-service | 8002 | 인기 목적지 날짜 + 관광지 목록 / 상세 + 리뷰 |
| admin-service | 8003 | 관리자: 로그인 + 목적지 / 관광지 CRUD |
| search-service | 8004 | 다국어 검색 |
| line-service | 8005 | 여행 라인 |
| order-service | 8006 | 주문 |
| flight-service | 8007 | 항공권 |
| hotel-service | 8008 | 호텔 |
| payment-service | 8009 | 결제 |
| Nginx 게이트웨이 | 8082→80 | `/api/user/`, `/api/booking/`, `/api/admin/`, `/api/search`, `/api/lines`, `/api/orders`, `/api/flights`, `/api/hotels`, `/api/payments` 프리픽스 라우팅 |
| MySQL | 3308→3306 | 데이터 소스 |
| Redis | 6381→6379 | 캐시 / 속도 제한 |
| OpenSearch | 9201→9200 | 다국어 검색 |

관리자 Flutter Web은 `apps/admin/`에 있으며, 개발 환경 기본 관리자 계정은 `admin@travel.local` / `Admin@123`(로컬 전용)입니다.

### 스크립트

| 스크립트 | 설명 |
|---|---|
| `scripts/opensearch_init.sh` | OpenSearch 인덱스를 멱등적으로 생성 (cjk 분석기) |
| `scripts/loadtest.sh` | 부하 테스트 |
| `scripts/cdn_setup.sh` / `cdn_upload.sh` | CDN 구성 및 업로드 (`--provider` 8클라우드 플러그인: cloudfront/aliyun/gcp/azure/cloudflare/tencent/huawei/bunny, 기본 `--dry-run`) |
| `scripts/release.sh` | 릴리스 프로세스 보조 |

---

## 후원하기

이 프로젝트가 도움이 되었다면, 작가에게 커피 한 잔을 대접해 주세요 ☕

<p align="center">
  <strong>微信支付(WeChat Pay)</strong> &nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp; <strong>支付宝(Alipay)</strong><br/>
  <img src="../../weixinpay.png" alt="微信支付 QR 코드" width="130" height="130" />
  &nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;
  <img src="../../alipay.png" alt="支付宝 QR 코드" width="130" height="130" />
</p>

### 글로벌 은행 송금(Global Bank Transfer)

**수취인 정보**

- 수취인 이름: WANG KEXUN
- 수취 계좌 번호: 881015918251

**수취 은행**

- ZA Bank SWIFT 코드: AABLHKHHXXX
- 은행 이름: ZA Bank Limited
- 은행 코드: 387
- 은행 주소: Core F, Cyberport 3, 100 Cyberport Road, Hong Kong

**국경 간 송금 중개 은행(필요 시)**

이 정보는 국경 간 송금 중개(중계) 은행 정보이며, 수취 은행 정보가 아닙니다. 송금 은행에 중개 은행 정보 제공이 필요한지 문의하시기 바랍니다.

홍콩 달러, 위안화, 미 달러 입금 시 중개 은행은 **Citibank**입니다 —

- 은행 이름: Citibank N.A. Hong Kong
- SWIFT 코드: CITIHKHXXXX
- 은행 코드: 006
- 지점 이름: Hong Kong Branch
- 지점 코드: 391
- 은행 주소: Citibank Tower, Citibank Plaza, 3 Garden Road, Central, Hong Kong

기타 통화 입금 시 중개 은행은 **BNY Mellon**입니다 —

- 은행 이름: THE BANK OF NEW YORK MELLON
- SWIFT 코드: IRVTUS3NXXX
- 은행 주소: THE BANK OF NEW YORK MELLON, 240 GREENWICH STREET, NEW YORK, United States

### 암호화폐 후원 (Crypto Donation)

이 프로젝트가 도움이 되셨다면, QR 코드를 스캔하여 후원해 주세요. 감사합니다!

| <img src="../../coin/1.jpg" width="200" alt="BNB Smart Chain (BEP20)"><br>**BNB Smart Chain (BEP20)**<br>`0x355d429f97511897ccb4e271ec888205f9ab6629` | <img src="../../coin/2.jpg" width="200" alt="Tron (TRC20)"><br>**Tron (TRC20)**<br>`TEdDHWLajt1XvqtPDWmQctdrJaC3pzZZzz` |
| <img src="../../coin/3.jpg" width="200" alt="Ethereum (ERC20)"><br>**Ethereum (ERC20)**<br>`0x355d429f97511897ccb4e271ec888205f9ab6629` | <img src="../../coin/4.jpg" width="200" alt="Aptos"><br>**Aptos**<br>`0x836e3780edfc3f7b2372b39e2a1a3a5d7adfaccd96c726f21cfde1b50dd68030` |
| <img src="../../coin/5.jpg" width="200" alt="Plasma"><br>**Plasma**<br>`0x355d429f97511897ccb4e271ec888205f9ab6629` | <img src="../../coin/6.jpg" width="200" alt="Polygon POS"><br>**Polygon POS**<br>`0x355d429f97511897ccb4e271ec888205f9ab6629` |
| <img src="../../coin/7.jpg" width="200" alt="Solana"><br>**Solana**<br>`2hfhboHdmdrYsY25XfQSsEWxq5ip4EQsR7f4AzSRMUyr` | <img src="../../coin/8.jpg" width="200" alt="The Open Network (TON)"><br>**The Open Network (TON)**<br>`UQB9kFQohzmXUir9QSSZq01iwl9aQZIDdBpNmDklljRtCoGK` |
| <img src="../../coin/9.jpg" width="200" alt="Arbitrum One"><br>**Arbitrum One**<br>`0x355d429f97511897ccb4e271ec888205f9ab6629` | <img src="../../coin/10.jpg" width="200" alt="AVAX C-Chain"><br>**AVAX C-Chain**<br>`0x355d429f97511897ccb4e271ec888205f9ab6629` |

