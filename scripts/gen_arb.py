#!/usr/bin/env python3
"""Generate all 13 ARB locale files for the Flutter app.

Usage: python3 scripts/gen_arb.py
Output: apps/flutter/assets/i18n/<lang>.arb (zh source + 12 translations)
All locales share the exact same key set; the script fails loudly otherwise.
"""
import json
import os

KEYS = [
    "appTitle", "tagline",
    "nav.home", "nav.search", "nav.bookings", "nav.profile",
    "home.heroTitle", "home.heroSubtitle", "home.popularDestinations",
    "search.placeholder", "search.button", "search.results",
    "destination.price", "destination.perNight", "destination.rating", "destination.bookNow",
    "booking.title", "booking.date", "booking.guests",
    "booking.status.pending", "booking.status.paid", "booking.status.cancelled", "booking.viewAll",
    "profile.title", "profile.login", "profile.register", "profile.lang",
    "common.loading", "common.error", "common.retry", "common.cancel",
    "common.currency", "common.noData", "common.welcome", "common.success",
]

T = {
    "zh": {
        "appTitle": "开放旅行", "tagline": "探索世界，一键预订",
        "nav.home": "首页", "nav.search": "搜索", "nav.bookings": "预订", "nav.profile": "我的",
        "home.heroTitle": "开启你的全球之旅", "home.heroSubtitle": "预订酒店、机票与目的地体验",
        "home.popularDestinations": "热门目的地",
        "search.placeholder": "搜索目的地、酒店或机票", "search.button": "搜索", "search.results": "搜索结果",
        "destination.price": "价格", "destination.perNight": "/晚", "destination.rating": "评分", "destination.bookNow": "立即预订",
        "booking.title": "我的预订", "booking.date": "日期", "booking.guests": "人数",
        "booking.status.pending": "待支付", "booking.status.paid": "已支付", "booking.status.cancelled": "已取消", "booking.viewAll": "查看全部",
        "profile.title": "个人中心", "profile.login": "登录", "profile.register": "注册", "profile.lang": "语言",
        "common.loading": "加载中…", "common.error": "出错了", "common.retry": "重试", "common.cancel": "取消",
        "common.currency": "¥", "common.noData": "暂无数据", "common.welcome": "欢迎", "common.success": "操作成功",
    },
    "en": {
        "appTitle": "Open Travel", "tagline": "Explore the world, book in one tap",
        "nav.home": "Home", "nav.search": "Search", "nav.bookings": "Bookings", "nav.profile": "Profile",
        "home.heroTitle": "Start Your Global Journey", "home.heroSubtitle": "Book hotels, flights and destination experiences",
        "home.popularDestinations": "Popular Destinations",
        "search.placeholder": "Search destinations, hotels or flights", "search.button": "Search", "search.results": "Search Results",
        "destination.price": "Price", "destination.perNight": "/night", "destination.rating": "Rating", "destination.bookNow": "Book Now",
        "booking.title": "My Bookings", "booking.date": "Date", "booking.guests": "Guests",
        "booking.status.pending": "Pending", "booking.status.paid": "Paid", "booking.status.cancelled": "Cancelled", "booking.viewAll": "View All",
        "profile.title": "Profile", "profile.login": "Log In", "profile.register": "Sign Up", "profile.lang": "Language",
        "common.loading": "Loading…", "common.error": "Something went wrong", "common.retry": "Retry", "common.cancel": "Cancel",
        "common.currency": "$", "common.noData": "No data", "common.welcome": "Welcome", "common.success": "Success",
    },
    "ja": {
        "appTitle": "オープントラベル", "tagline": "世界を探索、ワンタップで予約",
        "nav.home": "ホーム", "nav.search": "検索", "nav.bookings": "予約", "nav.profile": "マイページ",
        "home.heroTitle": "世界一周の旅を始めよう", "home.heroSubtitle": "ホテル、航空券、現地体験を予約",
        "home.popularDestinations": "人気の目的地",
        "search.placeholder": "目的地、ホテル、航空券を検索", "search.button": "検索", "search.results": "検索結果",
        "destination.price": "価格", "destination.perNight": "/泊", "destination.rating": "評価", "destination.bookNow": "今すぐ予約",
        "booking.title": "予約一覧", "booking.date": "日付", "booking.guests": "人数",
        "booking.status.pending": "未払い", "booking.status.paid": "支払い済み", "booking.status.cancelled": "キャンセル済み", "booking.viewAll": "すべて表示",
        "profile.title": "マイページ", "profile.login": "ログイン", "profile.register": "登録", "profile.lang": "言語",
        "common.loading": "読み込み中…", "common.error": "エラーが発生しました", "common.retry": "再試行", "common.cancel": "キャンセル",
        "common.currency": "¥", "common.noData": "データがありません", "common.welcome": "ようこそ", "common.success": "成功",
    },
    "ko": {
        "appTitle": "오픈트래블", "tagline": "세계를 탐험하고 한 번에 예약하세요",
        "nav.home": "홈", "nav.search": "검색", "nav.bookings": "예약", "nav.profile": "내 정보",
        "home.heroTitle": "당신의 글로벌 여행을 시작하세요", "home.heroSubtitle": "호텔, 항공권, 현지 체험 예약",
        "home.popularDestinations": "인기 여행지",
        "search.placeholder": "여행지, 호텔 또는 항공권 검색", "search.button": "검색", "search.results": "검색 결과",
        "destination.price": "가격", "destination.perNight": "/박", "destination.rating": "평점", "destination.bookNow": "지금 예약",
        "booking.title": "내 예약", "booking.date": "날짜", "booking.guests": "인원",
        "booking.status.pending": "결제 대기", "booking.status.paid": "결제 완료", "booking.status.cancelled": "취소됨", "booking.viewAll": "전체 보기",
        "profile.title": "내 정보", "profile.login": "로그인", "profile.register": "회원가입", "profile.lang": "언어",
        "common.loading": "불러오는 중…", "common.error": "오류가 발생했습니다", "common.retry": "다시 시도", "common.cancel": "취소",
        "common.currency": "₩", "common.noData": "데이터 없음", "common.welcome": "환영합니다", "common.success": "성공",
    },
    "ru": {
        "appTitle": "Опен Трэвел", "tagline": "Исследуйте мир — бронируйте в одно касание",
        "nav.home": "Главная", "nav.search": "Поиск", "nav.bookings": "Бронирования", "nav.profile": "Профиль",
        "home.heroTitle": "Начните ваше путешествие", "home.heroSubtitle": "Бронируйте отели, авиабилеты и экскурсии",
        "home.popularDestinations": "Популярные направления",
        "search.placeholder": "Поиск направлений, отелей или авиабилетов", "search.button": "Найти", "search.results": "Результаты поиска",
        "destination.price": "Цена", "destination.perNight": "/ночь", "destination.rating": "Рейтинг", "destination.bookNow": "Забронировать",
        "booking.title": "Мои бронирования", "booking.date": "Дата", "booking.guests": "Гости",
        "booking.status.pending": "Ожидает оплаты", "booking.status.paid": "Оплачено", "booking.status.cancelled": "Отменено", "booking.viewAll": "Смотреть все",
        "profile.title": "Профиль", "profile.login": "Войти", "profile.register": "Регистрация", "profile.lang": "Язык",
        "common.loading": "Загрузка…", "common.error": "Произошла ошибка", "common.retry": "Повторить", "common.cancel": "Отмена",
        "common.currency": "₽", "common.noData": "Нет данных", "common.welcome": "Добро пожаловать", "common.success": "Успешно",
    },
    "de": {
        "appTitle": "Open Travel", "tagline": "Entdecke die Welt, buche in einem Klick",
        "nav.home": "Start", "nav.search": "Suche", "nav.bookings": "Buchungen", "nav.profile": "Profil",
        "home.heroTitle": "Beginnen Sie Ihre Weltreise", "home.heroSubtitle": "Hotels, Flüge und Erlebnisse buchen",
        "home.popularDestinations": "Beliebte Reiseziele",
        "search.placeholder": "Reiseziele, Hotels oder Flüge suchen", "search.button": "Suchen", "search.results": "Suchergebnisse",
        "destination.price": "Preis", "destination.perNight": "/Nacht", "destination.rating": "Bewertung", "destination.bookNow": "Jetzt buchen",
        "booking.title": "Meine Buchungen", "booking.date": "Datum", "booking.guests": "Gäste",
        "booking.status.pending": "Ausstehend", "booking.status.paid": "Bezahlt", "booking.status.cancelled": "Storniert", "booking.viewAll": "Alle anzeigen",
        "profile.title": "Profil", "profile.login": "Anmelden", "profile.register": "Registrieren", "profile.lang": "Sprache",
        "common.loading": "Laden…", "common.error": "Etwas ist schiefgelaufen", "common.retry": "Erneut versuchen", "common.cancel": "Abbrechen",
        "common.currency": "€", "common.noData": "Keine Daten", "common.welcome": "Willkommen", "common.success": "Erfolg",
    },
    "fr": {
        "appTitle": "Open Travel", "tagline": "Explorez le monde, réservez en un geste",
        "nav.home": "Accueil", "nav.search": "Recherche", "nav.bookings": "Réservations", "nav.profile": "Profil",
        "home.heroTitle": "Commencez votre voyage autour du monde", "home.heroSubtitle": "Réservez hôtels, vols et expériences",
        "home.popularDestinations": "Destinations populaires",
        "search.placeholder": "Rechercher destinations, hôtels ou vols", "search.button": "Rechercher", "search.results": "Résultats de recherche",
        "destination.price": "Prix", "destination.perNight": "/nuit", "destination.rating": "Note", "destination.bookNow": "Réserver",
        "booking.title": "Mes réservations", "booking.date": "Date", "booking.guests": "Voyageurs",
        "booking.status.pending": "En attente", "booking.status.paid": "Payé", "booking.status.cancelled": "Annulé", "booking.viewAll": "Tout voir",
        "profile.title": "Profil", "profile.login": "Connexion", "profile.register": "S'inscrire", "profile.lang": "Langue",
        "common.loading": "Chargement…", "common.error": "Une erreur est survenue", "common.retry": "Réessayer", "common.cancel": "Annuler",
        "common.currency": "€", "common.noData": "Aucune donnée", "common.welcome": "Bienvenue", "common.success": "Succès",
    },
    "es": {
        "appTitle": "Open Travel", "tagline": "Explora el mundo, reserva con un toque",
        "nav.home": "Inicio", "nav.search": "Buscar", "nav.bookings": "Reservas", "nav.profile": "Perfil",
        "home.heroTitle": "Comienza tu viaje global", "home.heroSubtitle": "Reserva hoteles, vuelos y experiencias",
        "home.popularDestinations": "Destinos populares",
        "search.placeholder": "Buscar destinos, hoteles o vuelos", "search.button": "Buscar", "search.results": "Resultados de búsqueda",
        "destination.price": "Precio", "destination.perNight": "/noche", "destination.rating": "Valoración", "destination.bookNow": "Reservar ahora",
        "booking.title": "Mis reservas", "booking.date": "Fecha", "booking.guests": "Huéspedes",
        "booking.status.pending": "Pendiente", "booking.status.paid": "Pagado", "booking.status.cancelled": "Cancelado", "booking.viewAll": "Ver todo",
        "profile.title": "Perfil", "profile.login": "Iniciar sesión", "profile.register": "Registrarse", "profile.lang": "Idioma",
        "common.loading": "Cargando…", "common.error": "Algo salió mal", "common.retry": "Reintentar", "common.cancel": "Cancelar",
        "common.currency": "$", "common.noData": "Sin datos", "common.welcome": "Bienvenido", "common.success": "Éxito",
    },
    "pt": {
        "appTitle": "Open Travel", "tagline": "Explore o mundo, reserve em um toque",
        "nav.home": "Início", "nav.search": "Pesquisar", "nav.bookings": "Reservas", "nav.profile": "Perfil",
        "home.heroTitle": "Comece sua jornada global", "home.heroSubtitle": "Reserve hotéis, voos e experiências",
        "home.popularDestinations": "Destinos populares",
        "search.placeholder": "Pesquisar destinos, hotéis ou voos", "search.button": "Pesquisar", "search.results": "Resultados da pesquisa",
        "destination.price": "Preço", "destination.perNight": "/noite", "destination.rating": "Avaliação", "destination.bookNow": "Reservar agora",
        "booking.title": "Minhas reservas", "booking.date": "Data", "booking.guests": "Hóspedes",
        "booking.status.pending": "Pendente", "booking.status.paid": "Pago", "booking.status.cancelled": "Cancelado", "booking.viewAll": "Ver tudo",
        "profile.title": "Perfil", "profile.login": "Entrar", "profile.register": "Cadastrar", "profile.lang": "Idioma",
        "common.loading": "Carregando…", "common.error": "Algo deu errado", "common.retry": "Tentar novamente", "common.cancel": "Cancelar",
        "common.currency": "R$", "common.noData": "Sem dados", "common.welcome": "Bem-vindo", "common.success": "Sucesso",
    },
    "hi": {
        "appTitle": "ओपन ट्रैवल", "tagline": "दुनिया का अन्वेषण करें, एक टैप में बुक करें",
        "nav.home": "होम", "nav.search": "खोज", "nav.bookings": "बुकिंग", "nav.profile": "प्रोफ़ाइल",
        "home.heroTitle": "अपनी वैश्विक यात्रा शुरू करें", "home.heroSubtitle": "होटल, फ़्लाइट और अनुभव बुक करें",
        "home.popularDestinations": "लोकप्रिय गंतव्य",
        "search.placeholder": "गंतव्य, होटल या फ़्लाइट खोजें", "search.button": "खोजें", "search.results": "खोज परिणाम",
        "destination.price": "कीमत", "destination.perNight": "/रात", "destination.rating": "रेटिंग", "destination.bookNow": "अभी बुक करें",
        "booking.title": "मेरी बुकिंग", "booking.date": "तारीख़", "booking.guests": "मेहमान",
        "booking.status.pending": "लंबित", "booking.status.paid": "भुगतान किया गया", "booking.status.cancelled": "रद्द", "booking.viewAll": "सभी देखें",
        "profile.title": "प्रोफ़ाइल", "profile.login": "लॉग इन", "profile.register": "पंजीकरण", "profile.lang": "भाषा",
        "common.loading": "लोड हो रहा है…", "common.error": "कुछ गलत हो गया", "common.retry": "पुनः प्रयास करें", "common.cancel": "रद्द करें",
        "common.currency": "₹", "common.noData": "कोई डेटा नहीं", "common.welcome": "स्वागत है", "common.success": "सफल",
    },
    "ar": {
        "appTitle": "أوبن ترافل", "tagline": "استكشف العالم واحجز بلمسة واحدة",
        "nav.home": "الرئيسية", "nav.search": "بحث", "nav.bookings": "الحجوزات", "nav.profile": "حسابي",
        "home.heroTitle": "ابدأ رحلتك العالمية", "home.heroSubtitle": "احجز الفنادق والرحلات الجوية والتجارب",
        "home.popularDestinations": "الوجهات الشائعة",
        "search.placeholder": "ابحث عن وجهات أو فنادق أو رحلات جوية", "search.button": "بحث", "search.results": "نتائج البحث",
        "destination.price": "السعر", "destination.perNight": "/ليلة", "destination.rating": "التقييم", "destination.bookNow": "احجز الآن",
        "booking.title": "حجوزاتي", "booking.date": "التاريخ", "booking.guests": "الضيوف",
        "booking.status.pending": "قيد الانتظار", "booking.status.paid": "مدفوع", "booking.status.cancelled": "ملغي", "booking.viewAll": "عرض الكل",
        "profile.title": "حسابي", "profile.login": "تسجيل الدخول", "profile.register": "التسجيل", "profile.lang": "اللغة",
        "common.loading": "جارٍ التحميل…", "common.error": "حدث خطأ ما", "common.retry": "إعادة المحاولة", "common.cancel": "إلغاء",
        "common.currency": "ر.س", "common.noData": "لا توجد بيانات", "common.welcome": "مرحبًا بك", "common.success": "نجاح",
    },
    "bn": {
        "appTitle": "ওপেন ট্রাভেল", "tagline": "বিশ্ব অন্বেষণ করুন, এক ট্যাপে বুক করুন",
        "nav.home": "হোম", "nav.search": "অনুসন্ধান", "nav.bookings": "বুকিং", "nav.profile": "প্রোফাইল",
        "home.heroTitle": "আপনার বৈশ্বিক যাত্রা শুরু করুন", "home.heroSubtitle": "হোটেল, ফ্লাইট ও অভিজ্ঞতা বুক করুন",
        "home.popularDestinations": "জনপ্রিয় গন্তব্য",
        "search.placeholder": "গন্তব্য, হোটেল বা ফ্লাইট খুঁজুন", "search.button": "খুঁজুন", "search.results": "অনুসন্ধানের ফলাফল",
        "destination.price": "মূল্য", "destination.perNight": "/রাত", "destination.rating": "রেটিং", "destination.bookNow": "এখনই বুক করুন",
        "booking.title": "আমার বুকিং", "booking.date": "তারিখ", "booking.guests": "অতিথি",
        "booking.status.pending": "বাকি", "booking.status.paid": "পরিশোধিত", "booking.status.cancelled": "বাতিল", "booking.viewAll": "সব দেখুন",
        "profile.title": "প্রোফাইল", "profile.login": "লগ ইন", "profile.register": "নিবন্ধন", "profile.lang": "ভাষা",
        "common.loading": "লোড হচ্ছে…", "common.error": "কিছু ভুল হয়েছে", "common.retry": "আবার চেষ্টা করুন", "common.cancel": "বাতিল",
        "common.currency": "৳", "common.noData": "কোনো তথ্য নেই", "common.welcome": "স্বাগতম", "common.success": "সফল",
    },
    "id": {
        "appTitle": "Open Travel", "tagline": "Jelajahi dunia, pesan sekali sentuh",
        "nav.home": "Beranda", "nav.search": "Cari", "nav.bookings": "Pemesanan", "nav.profile": "Profil",
        "home.heroTitle": "Mulai Perjalanan Global Anda", "home.heroSubtitle": "Pesan hotel, tiket pesawat, dan pengalaman",
        "home.popularDestinations": "Destinasi Populer",
        "search.placeholder": "Cari destinasi, hotel, atau tiket pesawat", "search.button": "Cari", "search.results": "Hasil Pencarian",
        "destination.price": "Harga", "destination.perNight": "/malam", "destination.rating": "Rating", "destination.bookNow": "Pesan Sekarang",
        "booking.title": "Pemesanan Saya", "booking.date": "Tanggal", "booking.guests": "Tamu",
        "booking.status.pending": "Menunggu", "booking.status.paid": "Dibayar", "booking.status.cancelled": "Dibatalkan", "booking.viewAll": "Lihat Semua",
        "profile.title": "Profil", "profile.login": "Masuk", "profile.register": "Daftar", "profile.lang": "Bahasa",
        "common.loading": "Memuat…", "common.error": "Terjadi kesalahan", "common.retry": "Coba Lagi", "common.cancel": "Batal",
        "common.currency": "Rp", "common.noData": "Tidak ada data", "common.welcome": "Selamat Datang", "common.success": "Berhasil",
    },
}

EXPECTED = set(KEYS)
for lang, strings in T.items():
    missing = EXPECTED - set(strings)
    extra = set(strings) - EXPECTED
    if missing or extra:
        raise SystemExit(f"{lang}: missing={sorted(missing)} extra={sorted(extra)}")

OUT_DIR = os.path.normpath(os.path.join(os.path.dirname(__file__), "..", "apps", "flutter", "assets", "i18n"))
os.makedirs(OUT_DIR, exist_ok=True)

for lang in ["zh", "en", "ja", "ko", "ru", "de", "fr", "es", "pt", "hi", "ar", "bn", "id"]:
    strings = T[lang]
    arb = {"@@locale": lang}
    for key in KEYS:
        arb[key] = strings[key]
        arb[f"@{key}"] = {"description": key}
    path = os.path.join(OUT_DIR, f"{lang}.arb")
    with open(path, "w", encoding="utf-8") as f:
        json.dump(arb, f, ensure_ascii=False, indent=2)
        f.write("\n")
    print(f"wrote {path} ({len(KEYS)} keys)")

# key-set consistency check across all 13 files
sets = {}
for name in sorted(os.listdir(OUT_DIR)):
    with open(os.path.join(OUT_DIR, name), encoding="utf-8") as f:
        d = json.load(f)
    sets[name] = (d["@@locale"], {k for k in d if not k.startswith("@")})
base = sets["zh.arb"][1]
ok = True
for name, (loc, ks) in sets.items():
    if ks != base:
        print(f"KEY DIFF {name}: missing={base - ks} extra={ks - base}")
        ok = False
print(f"files={len(sets)} keys_per_file={len(base)} locale_check={'OK' if ok else 'FAIL'}")
