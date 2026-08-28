# Open Travel — বৈশ্বিক ভ্রমণ প্ল্যাটফর্ম

[简体中文](../../README.md) | [English](README.md) | [日本語](ja/README.md) | [한국어](ko/README.md) | [Русский](ru/README.md) | [Deutsch](de/README.md) | [Français](fr/README.md) | [Español](es/README.md) | [Português](pt/README.md) | [हिन्दी](hi/README.md) | [العربية](ar/README.md) | [বাংলা](bn/README.md) | [Bahasa Indonesia](id/README.md)

> একটি বৈশ্বিক ভ্রমণ বুকিং প্ল্যাটফর্ম: Rust মাইক্রোসার্ভিস ব্যাকএন্ড + Flutter / HarmonyOS মাল্টি-প্ল্যাটফর্ম ক্লায়েন্ট, **12+ ভাষা**, আন্তর্জাতিক পেমেন্ট এবং বহুভাষিক অনুসন্ধান সমর্থন করে।

## প্রকল্প পরিচিতি

Open Travel একটি বৈশ্বিক ভ্রমণ প্ল্যাটফর্ম মনোরেপো, যা **e-cat (একটি বিড়াল)** — [go-kratos/kratos](https://github.com/go-kratos/kratos) v3-এর সমতুল্য **Rust মাইক্রোসার্ভিস ফ্রেমওয়ার্ক** (v3.0.2 · 51 crates) — দিয়ে উচ্চ-পারফরম্যান্স ব্যাকএন্ড তৈরি করে, সাথে Flutter মাল্টি-প্ল্যাটফর্ম এবং HarmonyOS নেটিভ ক্লায়েন্ট, বিশ্বব্যাপী ব্যবহারকারীদের জন্য একীভূত ভ্রমণ বুকিং অভিজ্ঞতা প্রদান করে।

| মাত্রা | বিবরণ |
| :--- | :--- |
| **ব্যাকএন্ড ফ্রেমওয়ার্ক** | e-cat (Rust): HTTP/axum + gRPC/tonic, 51 crates মাইক্রোসার্ভিস ইকোসিস্টেম |
| **মাল্টি-প্ল্যাটফর্ম ক্লায়েন্ট** | `apps/client/flutter` (iOS / Android / Web / Desktop), `apps/client/harmonyos` (HarmonyOS) |
| **ডেটাবেস** | MySQL (ডেটাবেস `travel`, টেবিল প্রিফিক্স `travel_`) + Redis ক্যাশ + OpenSearch বহুভাষিক অনুসন্ধান |
| **নিরাপত্তা** | ecat-security / ecat-auth (JWT) / ecat-tls: অথেনটিকেশন, অডিট, রেট লিমিটিং, ইনজেকশন সুরক্ষা |
| **আন্তর্জাতিককরণ** | 12+ ভাষার ARB ভাষা প্যাক, RTL সাপোর্ট, OpenSearch বহুভাষিক টোকেনাইজেশন |
| **পেমেন্ট** | WeChat Pay, Alipay |

## মূল বৈশিষ্ট্য

- 🏨 গন্তব্য / হোটেল / ফ্লাইট টিকিটের বহুভাষিক অনুসন্ধান ও বুকিং
- 🌍 12+ ভাষার স্বতন্ত্র অভিযোজন (চীনা, ইংরেজি, জাপানি, কোরিয়ান, আরবি, স্প্যানিশ, ফরাসি, জার্মান...)
- 💳 আন্তর্জাতিক পেমেন্ট (WeChat Pay / Alipay)
- 🔐 গভীর প্রতিরক্ষা: TLS 1.3, JWT অথেনটিকেশন, অডিট লগ, ইনপুট ফিল্টারিং, রেট লিমিটিং
- 📱 সব প্ল্যাটফর্মে একই রকম অভিজ্ঞতা: Flutter (iOS/Android/Web/Desktop) + HarmonyOS

## আর্কিটেকচার ডায়াগ্রাম

![আর্কিটেকচার ডায়াগ্রাম](../../svg/bn/architecture.svg)

## ফিচার ডায়াগ্রাম

![ফিচার ডায়াগ্রাম](../../svg/bn/features.svg)

## প্রজেক্ট ডায়াগ্রাম

![প্রজেক্ট ডায়াগ্রাম](../../svg/bn/project.svg)

## রিকোয়েস্ট সাইকেল ডায়াগ্রাম

![রিকোয়েস্ট সাইকেল ডায়াগ্রাম](../../svg/bn/request-cycle.svg)

## সিকিউরিটি আর্কিটেকচার ডায়াগ্রাম

![সিকিউরিটি আর্কিটেকচার ডায়াগ্রাম](../../svg/bn/security-architecture.svg)

## প্রজেক্ট স্ট্রাকচার ডায়াগ্রাম

![প্রজেক্ট স্ট্রাকচার ডায়াগ্রাম](../../svg/bn/project-structure.svg)

## প্রজেক্ট স্ট্রাকচার

```
open-travel/
├── apps/                  # মাল্টি-প্ল্যাটফর্ম ক্লায়েন্ট ডিরেক্টরি
│   ├── flutter/           # Flutter: iOS / Android / Web / Desktop (12+ ভাষার i18n)
│   └── harmonyos/         # HarmonyOS নেটিভ ক্লায়েন্ট
├── e-cat/                 # e-cat Rust মাইক্রোসার্ভিস ফ্রেমওয়ার্ক (51 crates)
├── docs/                  # প্রজেক্ট প্ল্যানিং, ডায়াগ্রাম (SVG), পেমেন্ট QR কোড
├── config/                # পরিবেশ ও ডিপ্লয়মেন্ট কনফিগারেশন
└── README.md
```

## ডেটাবেস

- ডেটাবেসের নাম: `travel`
- টেবিল প্রিফিক্স: `travel_` (যেমন `travel_users`, `travel_orders`, `travel_reviews`)
- সহযোগী স্টোরেজ: Redis (সেশন / জনপ্রিয় ক্যাশ), OpenSearch (বহুভাষিক সার্চ ইনডেক্স)

> বিস্তারিত প্রযুক্তিগত পরিকল্পনা দেখুন [docs/travel-project-planning.md](../../travel-project-planning.md)।

## দ্রুত শুরু

```bash
cd e-cat
cargo check -p user-service -p booking-service -p admin-service   # বিজনেস সার্ভিসের কম্পাইল চেক
```

| সার্ভিস | পোর্ট | বিবরণ |
|---|---|---|
| user-service | 8001 | ইউজার নিবন্ধন / লগইন / প্রোফাইল |
| booking-service | 8002 | জনপ্রিয় গন্তব্যের তারিখ + আকর্ষণ তালিকা / বিস্তারিত |
| admin-service | 8003 | অ্যাডমিন: লগইন + গন্তব্য / আকর্ষণ CRUD |
| Nginx গেটওয়ে | 8082→80 | `/api/user/`, `/api/booking/`, `/api/admin/` প্রিফিক্স রাউটিং |
| MySQL | 3308→3306 | ডেটা সোর্স |
| Redis | 6381→6379 | ক্যাশ / রেট লিমিটিং |
| OpenSearch | 9201→9200 | বহুভাষিক অনুসন্ধান |

অ্যাডমিন Flutter Web অ্যাপ `apps/admin/`-এ রয়েছে; ডেভেলপমেন্টের ডিফল্ট অ্যাডমিন অ্যাকাউন্ট `admin@travel.local` / `Admin@123` (শুধুমাত্র লোকাল ব্যবহার)।

---

## আমাদের সমর্থন করুন

যদি এই প্রজেক্টটি আপনার কাজে লাগে, লেখককে এক কাপ কফি খাওয়াতে পারেন ☕

<p align="center">
  <strong>WeChat Pay</strong> &nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp; <strong>Alipay</strong><br/>
  <img src="../../weixinpay.png" alt="WeChat Pay QR কোড" width="130" height="130" />
  &nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;
  <img src="../../alipay.png" alt="Alipay QR কোড" width="130" height="130" />
</p>

### গ্লোবাল ব্যাংক ট্রান্সফার দান (Global Bank Transfer)

**প্রাপকের তথ্য**

- প্রাপকের নাম: WANG KEXUN
- প্রাপকের অ্যাকাউন্ট নম্বর: 881015918251

**প্রাপক ব্যাংক**

- ZA Bank SWIFT কোড: AABLHKHHXXX
- ব্যাংকের নাম: ZA Bank Limited
- ব্যাংক কোড: 387
- ব্যাংকের ঠিকানা: Core F, Cyberport 3, 100 Cyberport Road, Hong Kong

**ক্রস-বর্ডার রেমিট্যান্স করেসপনডেন্ট ব্যাংক (যদি প্রয়োজন হয়)**

দয়া করে লক্ষ্য করুন, এটি ক্রস-বর্ডার রেমিট্যান্সের জন্য করেসপনডেন্ট (মধ্যস্থ) ব্যাংকের তথ্য, প্রাপক ব্যাংকের তথ্য নয়। রেমিট্যান্স পাঠানোর ব্যাংককে জিজ্ঞাসা করুন যে ক্রস-বর্ডার করেসপনডেন্ট ব্যাংকের তথ্য প্রদান করা প্রয়োজন কি না।

HKD, RMB এবং USD-এর জন্য করেসপনডেন্ট ব্যাংক হল **Citibank** —

- ব্যাংকের নাম: Citibank N.A. Hong Kong
- SWIFT কোড: CITIHKHXXXX
- ব্যাংক কোড: 006
- শাখার নাম: Hong Kong Branch
- শাখা কোড: 391
- ব্যাংকের ঠিকানা: Citibank Tower, Citibank Plaza, 3 Garden Road, Central, Hong Kong

অন্যান্য মুদ্রায় রেমিট্যান্সের জন্য করেসপনডেন্ট ব্যাংক হল **BNY Mellon** —

- ব্যাংকের নাম: THE BANK OF NEW YORK MELLON
- SWIFT কোড: IRVTUS3NXXX
- ব্যাংকের ঠিকানা: THE BANK OF NEW YORK MELLON, 240 GREENWICH STREET, NEW YORK, United States

### ক্রিপ্টো দান (Crypto Donation)

এই প্রকল্পটি আপনার কাজে লাগলে, দান করতে QR কোড স্ক্যান করুন, ধন্যবাদ!

| <img src="../../coin/1.jpg" width="200" alt="BNB Smart Chain (BEP20)"><br>**BNB Smart Chain (BEP20)**<br>`0x355d429f97511897ccb4e271ec888205f9ab6629` | <img src="../../coin/2.jpg" width="200" alt="Tron (TRC20)"><br>**Tron (TRC20)**<br>`TEdDHWLajt1XvqtPDWmQctdrJaC3pzZZzz` |
| <img src="../../coin/3.jpg" width="200" alt="Ethereum (ERC20)"><br>**Ethereum (ERC20)**<br>`0x355d429f97511897ccb4e271ec888205f9ab6629` | <img src="../../coin/4.jpg" width="200" alt="Aptos"><br>**Aptos**<br>`0x836e3780edfc3f7b2372b39e2a1a3a5d7adfaccd96c726f21cfde1b50dd68030` |
| <img src="../../coin/5.jpg" width="200" alt="Plasma"><br>**Plasma**<br>`0x355d429f97511897ccb4e271ec888205f9ab6629` | <img src="../../coin/6.jpg" width="200" alt="Polygon POS"><br>**Polygon POS**<br>`0x355d429f97511897ccb4e271ec888205f9ab6629` |
| <img src="../../coin/7.jpg" width="200" alt="Solana"><br>**Solana**<br>`2hfhboHdmdrYsY25XfQSsEWxq5ip4EQsR7f4AzSRMUyr` | <img src="../../coin/8.jpg" width="200" alt="The Open Network (TON)"><br>**The Open Network (TON)**<br>`UQB9kFQohzmXUir9QSSZq01iwl9aQZIDdBpNmDklljRtCoGK` |
| <img src="../../coin/9.jpg" width="200" alt="Arbitrum One"><br>**Arbitrum One**<br>`0x355d429f97511897ccb4e271ec888205f9ab6629` | <img src="../../coin/10.jpg" width="200" alt="AVAX C-Chain"><br>**AVAX C-Chain**<br>`0x355d429f97511897ccb4e271ec888205f9ab6629` |

