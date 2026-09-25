[简体中文](../../README.md) | [English](README.md) | [日本語](ja/README.md) | [한국어](ko/README.md) | [Русский](ru/README.md) | [Deutsch](de/README.md) | [Français](fr/README.md) | [Español](es/README.md) | [Português](pt/README.md) | [हिन्दी](hi/README.md) | [العربية](ar/README.md) | [বাংলা](bn/README.md) | [Bahasa Indonesia](id/README.md)

---

# Open Travel — वैश्विक यात्रा प्लेटफ़ॉर्म

<p align="center"><img src="../../mascot.svg" alt="Nannan 南南 — Open Travel शुभंकर" width="180"></p>


> वैश्विक उपयोगकर्ताओं के लिए एक यात्रा बुकिंग प्लेटफ़ॉर्म: Rust माइक्रोसर्विस बैकएंड + Flutter / HarmonyOS मल्टी-प्लेटफ़ॉर्म क्लाइंट, **12+ भाषाओं**, अंतर्राष्ट्रीय भुगतान और बहुभाषी खोज का समर्थन करता है।

## परियोजना परिचय

Open Travel एक वैश्विक यात्रा प्लेटफ़ॉर्म monorepo है, जो **e-cat (एक बिल्ली)** — [go-kratos/kratos](https://github.com/go-kratos/kratos) v3 से प्रेरित एक **Rust माइक्रोसर्विस फ्रेमवर्क** (v3.0.3 · 52 crates) — का उपयोग करके उच्च-प्रदर्शन बैकएंड बनाता है, साथ ही Flutter मल्टी-प्लेटफ़ॉर्म और HarmonyOS नेटिव क्लाइंट के साथ, वैश्विक उपयोगकर्ताओं के लिए एकीकृत यात्रा बुकिंग अनुभव प्रदान करता है।

| आयाम | विवरण |
| :--- | :--- |
| **बैकएंड फ्रेमवर्क** | e-cat (Rust): HTTP/axum + gRPC/tonic, 52 crates माइक्रोसर्विस इकोसिस्टम |
| **मल्टी-प्लेटफ़ॉर्म क्लाइंट** | `apps/client/flutter` (iOS / Android / Web / Desktop), `apps/client/harmonyos` (HarmonyOS), `apps/admin` (Flutter Web एडमिन कंसोल) |
| **डेटाबेस** | MySQL (डेटाबेस `travel`, टेबल प्रीफ़िक्स `travel_`) + Redis कैश + OpenSearch बहुभाषी खोज |
| **सुरक्षा** | ecat-security / ecat-auth (JWT) / ecat-tls: प्रमाणीकरण, ऑडिट, रेट लिमिटिंग, इंजेक्शन रोकथाम |
| **अंतर्राष्ट्रीयकरण** | 12+ भाषाओं के ARB भाषा पैक, RTL समर्थन, OpenSearch बहुभाषी टोकनाइज़ेशन |
| **भुगतान** | WeChat Pay, Alipay |

## परियोजना शुभंकर «南南»

एक कम्पास जिन: गोल डायल ही उसका शरीर है, बेज़ल पर **12 खाने = 12+ भाषाएँ**, सिर पर एक सुई सीधी खड़ी है, दाहिना हाथ आवर्धक लेंस उठाकर गंतव्य ढूँढ़ता है, और ऊपर-बाएँ कोने में एक कागज़ का हवाई जहाज़ उड़ान-मार्ग खींचता चलता है। सपाट ज्यामितीय शैली (ठोस रंग-खंड + पतली रेखाएँ, न ग्रेडिएंट न अर्ध-पारदर्शिता), वेक्टर स्रोत और पूरी जानकारी देखें [`docs/mascot.svg`](../../mascot.svg)।

| स्थान | रूप |
| :--- | :--- |
| `docs/mascot.svg` | **एकमात्र वेक्टर स्रोत** (पूर्ण आकृति 512×512) |
| `apps/*/web/favicon.svg` | ब्राउज़र टैब आइकन (डायल का क्लोज़-अप संस्करण, 16px पर भी पहचानने योग्य; पुराने ब्राउज़रों के लिए `favicon.png` 16px विकल्प) |
| `apps/*/web/icons/Icon-*.png` | PWA / होम स्क्रीन आइकन 192·512 (maskable सुरक्षित क्षेत्र संस्करण सहित) |
| `apps/*/assets/mascot.png` | Flutter ऐप के भीतर प्रदर्शन (एडमिन लॉगिन पृष्ठ, क्लाइंट प्रोफ़ाइल पृष्ठ) |
| `apps/client/harmonyos/.../media/mascot.svg` | HarmonyOS ऐप के भीतर (`Image` से मूल SVG रेंडरिंग; मोबाइल के लिए `opacity` रहित संस्करण) |
| सभी README / crate दस्तावेज़ | पृष्ठ शीर्ष पर ब्रांड स्थान |

> शक्ल बदलने के लिए केवल `docs/mascot.svg` बदलें, बाकी सब व्युत्पन्न हैं: favicon उसके डायल का क्लोज़-अप क्रॉप है (भुजाएँ / आवर्धक लेंस / पैर / उड़ान-मार्ग जैसी वे रेखाएँ हटाकर जो छोटे आकार में धुँधली पड़ जाती हैं), और HarmonyOS संस्करण अर्ध-पारदर्शी रंगों को पहले ही ठोस रंगों में बदल देता है और सभी `opacity` हटा देता है ताकि मोबाइल SVG रेंडरर में फिट हो सके। सभी आकृतियाँ मौलिक हैं, किसी तीसरे पक्ष की सामग्री पर निर्भरता नहीं।

## मुख्य विशेषताएँ

- 🏨 गंतव्यों / होटलों / उड़ानों की बहुभाषी खोज और बुकिंग
- 🌍 12+ भाषाओं का स्वतंत्र अनुकूलन (चीनी, अंग्रेज़ी, जापानी, कोरियाई, अरबी, स्पेनिश, फ्रेंच, जर्मन…)
- 💳 अंतर्राष्ट्रीय भुगतान (WeChat Pay / Alipay)
- 🔐 गहन सुरक्षा: TLS 1.3, JWT प्रमाणीकरण, ऑडिट लॉग, इनपुट फ़िल्टरिंग, रेट लिमिटिंग, भुगतान कॉलबैक HMAC सत्यापन, आंतरिक सेवा प्रमाणीकरण
- 📱 मल्टी-प्लेटफ़ॉर्म पर समान अनुभव: Flutter (iOS/Android/Web/Desktop) + HarmonyOS

## आर्किटेक्चर डिज़ाइन आरेख

![आर्किटेक्चर डिज़ाइन आरेख](../../svg/hi/architecture.svg)

## कार्यात्मकता डिज़ाइन आरेख

![कार्यात्मकता डिज़ाइन आरेख](../../svg/hi/features.svg)

## परियोजना संरचना आरेख

![परियोजना संरचना आरेख](../../svg/hi/project.svg)

## अनुरोध जीवनचक्र आरेख

![अनुरोध जीवनचक्र आरेख](../../svg/hi/request-cycle.svg)

## सुरक्षा आर्किटेक्चर आरेख

![सुरक्षा आर्किटेक्चर आरेख](../../svg/hi/security-architecture.svg)

## परियोजना संरचना

```
open-travel/
├── apps/                  # मल्टी-प्लेटफ़ॉर्म क्लाइंट और एडमिन पैनल
│   ├── client/
│   │   ├── flutter/       # Flutter: iOS / Android / Web / Desktop (12+ भाषाओं में i18n, web/favicon.svg शुभंकर आइकन)
│   │   └── harmonyos/     # HarmonyOS नेटिव क्लाइंट
│   └── admin/             # Flutter Web एडमिन पैनल
├── e-cat/                 # e-cat फ्रेमवर्क + बिजनेस सेवाएं (एक ही Cargo workspace)
│   ├── ecat*/             # 52 ecat-* फ्रेमवर्क क्रेट
│   ├── ecat/              # मुख्य फ्रेमवर्क क्रेट: फेकाडे + बिजनेस मॉड्यूल (src/business/) + सर्विस एंट्री (src/bin/ 9 सेवाएँ)
│   ├── config/            # फ्रेमवर्क कॉन्फ़िग उदाहरण
│   ├── examples/          # फ्रेमवर्क उदाहरण प्रोजेक्ट
│   └── CHANGELOG.md       # फ्रेमवर्क + प्रोजेक्ट चेंजलॉग
├── docs/                  # तकनीकी दस्तावेज़
│   ├── api.md             # API संदर्भ (एंडपॉइंट, प्रमाणीकरण, रेट लिमिटिंग)
│   ├── mascot.svg         # शुभंकर «南南» (एकमात्र वेक्टर स्रोत, favicon व अन्य आइकन इससे व्युत्पन्न)
│   ├── svg/               # आर्किटेक्चर / फीचर / लाइफसाइकल / सुरक्षा / संरचना आरेख (12 भाषाओं के अनुवाद सहित)
│   ├── i18n/              # 12 भाषाओं में README
│   └── coin/              # दान QR कोड
├── config/                # पर्यावरण और डिप्लॉयमेंट कॉन्फ़िगरेशन (nginx.conf, docker-compose.yml, schema.sql)
├── scripts/               # इंस्टॉल / डिप्लॉय / हेल्थ चेक / लोड टेस्ट / CDN / सीड डेटा
├── .github/workflows/     # CI
└── README.md
```

## डेटाबेस

- डेटाबेस नाम: `travel`
- टेबल प्रीफ़िक्स: `travel_` (उदाहरण: `travel_users`, `travel_orders`, `travel_reviews`)
- सहायक स्टोरेज: Redis (सत्र / लोकप्रिय कैश), OpenSearch (बहुभाषी खोज इंडेक्स)

> विस्तृत तकनीकी योजना के लिए देखें [docs/travel-project-planning.md](../../travel-project-planning.md)।

## वन-क्लिक इंस्टॉल

आवश्यकताएँ: Docker + Docker Compose (v2)। (Rust टूलचेन केवल सोर्स से बिल्ड करने के लिए आवश्यक है।)

```bash
git clone <repo-url> && cd open-travel
./scripts/install.sh
```

स्क्रिप्ट स्वचालित रूप से: पर्यावरण की जाँच → सभी सेवाएँ बिल्ड व शुरू करना (MySQL / Redis / OpenSearch / Kafka / 9 माइक्रोसर्विस / Nginx गेटवे) → सर्च इंडेक्स इनिशियलाइज़ → स्वास्थ्य जाँच, फिर एक्सेस URL और डिफ़ॉल्ट एडमिन खाता `admin@travel.local` / `Admin@123` प्रिंट करती है।

## त्वरित आरंभ

```bash
cd e-cat
cargo check -p ecat --bins   # व्यावसायिक सेवाओं की कंपाइल जाँच
```

| सेवा | पोर्ट | विवरण |
|---|---|---|
| user-service | 8001 | उपयोगकर्ता पंजीकरण / लॉगिन / प्रोफ़ाइल |
| booking-service | 8002 | लोकप्रिय गंतव्य तिथियाँ + आकर्षण सूची / विवरण + समीक्षाएं |
| admin-service | 8003 | व्यवस्थापन: लॉगिन + गंतव्य / आकर्षण CRUD |
| search-service | 8004 | बहुभाषी खोज |
| line-service | 8005 | यात्रा लाइनें |
| order-service | 8006 | ऑर्डर |
| flight-service | 8007 | उड़ानें |
| hotel-service | 8008 | होटल |
| payment-service | 8009 | भुगतान |
| Nginx गेटवे | 8082→80 | `/api/user/`, `/api/booking/`, `/api/admin/`, `/api/search`, `/api/lines`, `/api/orders`, `/api/flights`, `/api/hotels`, `/api/payments` प्रीफिक्स रूटिंग |
| MySQL | 3308→3306 | डेटा स्रोत |
| Redis | 6381→6379 | कैश / रेट लिमिटिंग |
| OpenSearch | 9201→9200 | बहुभाषी खोज |

व्यवस्थापन Flutter Web ऐप `apps/admin/` में है; विकास के लिए डिफ़ॉल्ट व्यवस्थापक खाता `admin@travel.local` / `Admin@123` है (केवल स्थानीय उपयोग)।

### स्क्रिप्ट्स

| स्क्रिप्ट | विवरण |
|---|---|
| `scripts/opensearch_init.sh` | OpenSearch इंडेक्स को इडेम्पोटेंट रूप से बनाता है (cjk विश्लेषक) |
| `scripts/loadtest.sh` | लोड परीक्षण |
| `scripts/cdn_setup.sh` / `cdn_upload.sh` | CDN कॉन्फ़िगरेशन और अपलोड (`--provider` आठ-क्लाउड प्लगइन: cloudfront/aliyun/gcp/azure/cloudflare/tencent/huawei/bunny, डिफ़ॉल्ट रूप से `--dry-run`) |
| `scripts/release.sh` | रिलीज़ प्रक्रिया सहायक |

---

## हमें समर्थन दें

अगर यह परियोजना आपके लिए उपयोगी है, तो लेखक को एक कॉफ़ी पिलाएँ ☕

<p align="center">
  <strong>微信支付（WeChat Pay）</strong> &nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp; <strong>支付宝（Alipay）</strong><br/>
  <img src="../../weixinpay.png" alt="WeChat Pay QR कोड" width="130" height="130" />
  &nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;
  <img src="../../alipay.png" alt="Alipay QR कोड" width="130" height="130" />
</p>

### वैश्विक बैंक ट्रांसफर (Global Bank Transfer) दान

**प्राप्तकर्ता (Beneficiary) जानकारी**

- प्राप्तकर्ता का नाम: WANG KEXUN
- प्राप्तकर्ता खाता संख्या: 881015918251

**प्राप्तकर्ता बैंक**

- ZA Bank SWIFT Code: AABLHKHHXXX
- बैंक का नाम: ZA Bank Limited
- बैंक कोड: 387
- बैंक का पता: Core F, Cyberport 3, 100 Cyberport Road, Hong Kong

**अंतर्राष्ट्रीय रेमिटेंस के लिए संवाददाता बैंक (यदि आवश्यक हो)**

कृपया ध्यान दें: यह अंतर्राष्ट्रीय रेमिटेंस के लिए संवाददाता बैंक (मध्यस्थ बैंक) की जानकारी है, प्राप्तकर्ता बैंक की नहीं। कृपया अपने रेमिटेंस बैंक से पूछें कि क्या संवाददाता बैंक की जानकारी प्रदान करना आवश्यक है।

हांगकांग डॉलर, रेनमिन्बी और अमेरिकी डॉलर जमा के लिए संवाददाता बैंक **Citibank** है —

- बैंक का नाम: Citibank N.A. Hong Kong
- SWIFT Code: CITIHKHXXXX
- बैंक कोड: 006
- शाखा का नाम: Hong Kong Branch
- शाखा कोड: 391
- बैंक का पता: Citibank Tower, Citibank Plaza, 3 Garden Road, Central, Hong Kong

अन्य मुद्राओं में जमा के लिए संवाददाता बैंक **BNY Mellon** है —

- बैंक का नाम: THE BANK OF NEW YORK MELLON
- SWIFT Code: IRVTUS3NXXX
- बैंक का पता: THE BANK OF NEW YORK MELLON, 240 GREENWICH STREET, NEW YORK, United States

### क्रिप्टो दान (Crypto Donation)

यदि यह प्रोजेक्ट आपके काम आए, तो दान करने के लिए QR कोड स्कैन करें, धन्यवाद!

| <img src="../../coin/1.jpg" width="200" alt="BNB Smart Chain (BEP20)"><br>**BNB Smart Chain (BEP20)**<br>`0x355d429f97511897ccb4e271ec888205f9ab6629` | <img src="../../coin/2.jpg" width="200" alt="Tron (TRC20)"><br>**Tron (TRC20)**<br>`TEdDHWLajt1XvqtPDWmQctdrJaC3pzZZzz` |
| <img src="../../coin/3.jpg" width="200" alt="Ethereum (ERC20)"><br>**Ethereum (ERC20)**<br>`0x355d429f97511897ccb4e271ec888205f9ab6629` | <img src="../../coin/4.jpg" width="200" alt="Aptos"><br>**Aptos**<br>`0x836e3780edfc3f7b2372b39e2a1a3a5d7adfaccd96c726f21cfde1b50dd68030` |
| <img src="../../coin/5.jpg" width="200" alt="Plasma"><br>**Plasma**<br>`0x355d429f97511897ccb4e271ec888205f9ab6629` | <img src="../../coin/6.jpg" width="200" alt="Polygon POS"><br>**Polygon POS**<br>`0x355d429f97511897ccb4e271ec888205f9ab6629` |
| <img src="../../coin/7.jpg" width="200" alt="Solana"><br>**Solana**<br>`2hfhboHdmdrYsY25XfQSsEWxq5ip4EQsR7f4AzSRMUyr` | <img src="../../coin/8.jpg" width="200" alt="The Open Network (TON)"><br>**The Open Network (TON)**<br>`UQB9kFQohzmXUir9QSSZq01iwl9aQZIDdBpNmDklljRtCoGK` |
| <img src="../../coin/9.jpg" width="200" alt="Arbitrum One"><br>**Arbitrum One**<br>`0x355d429f97511897ccb4e271ec888205f9ab6629` | <img src="../../coin/10.jpg" width="200" alt="AVAX C-Chain"><br>**AVAX C-Chain**<br>`0x355d429f97511897ccb4e271ec888205f9ab6629` |

