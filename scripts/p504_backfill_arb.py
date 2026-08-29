#!/usr/bin/env python3
"""P5-04: backfill 31 flight/hotel/payment/order keys into 11 non-en/zh ARBs.
Rebuilds all 13 ARBs canonically: @@locale + real keys in en order, one-line
@key metadata, no duplicate/@@ metadata. Values preserved; only adds missing.
"""
import json

ARB = "/home/wwwroot/open-travel/apps/client/flutter/assets/i18n"
LANGS = ["en", "zh", "ja", "ko", "ru", "de", "fr", "es", "pt", "hi", "ar", "bn", "id"]

NEW = {
    "ja": {
        "flight.title": "航空券", "flight.from": "出発地", "flight.to": "到着地",
        "flight.date": "出発日", "flight.cabin": "クラス", "flight.cabin.all": "全クラス",
        "flight.cabin.economy": "エコノミー", "flight.cabin.business": "ビジネス", "flight.cabin.first": "ファースト",
        "flight.search": "航空券を検索", "flight.seatsLeft": "残席：",
        "flight.noFlights": "この路線の便はありません",
        "flight.hint": "出発地と到着地の都市コードを入力",
        "hotel.title": "ホテル", "hotel.city": "都市コード", "hotel.star": "星評価",
        "hotel.allStars": "すべての星", "hotel.starUnit": "つ星",
        "hotel.search": "ホテルを検索", "hotel.noHotels": "この都市にホテルはありません",
        "hotel.hint": "都市コードを入力", "hotel.rooms": "部屋数",
        "hotel.breakfast": "朝食込み", "hotel.inventory": "残り：",
        "hotel.book": "予約する", "hotel.checkIn": "チェックイン", "hotel.checkOut": "チェックアウト",
        "order.product": "商品", "payment.title": "支払い",
        "payment.channels": "支払い方法を選択", "payment.goPay": "今すぐ支払う",
        "payment.checkoutHint": "ブラウザで支払いを完了してください。このページは自動的に更新されます",
        "payment.success": "支払いが完了しました",
        "payment.timeout": "支払い結果を確認できませんでした。後で注文を確認してください",
    },
    "ko": {
        "flight.title": "항공권", "flight.from": "출발지", "flight.to": "도착지",
        "flight.date": "출발 날짜", "flight.cabin": "좌석 등급", "flight.cabin.all": "전체 등급",
        "flight.cabin.economy": "이코노미", "flight.cabin.business": "비즈니스", "flight.cabin.first": "퍼스트",
        "flight.search": "항공권 검색", "flight.seatsLeft": "잔여 좌석:",
        "flight.noFlights": "이 노선에는 항공편이 없습니다",
        "flight.hint": "출발지와 도착지의 도시 코드를 입력하세요",
        "hotel.title": "호텔", "hotel.city": "도시 코드", "hotel.star": "별점",
        "hotel.allStars": "전체 별점", "hotel.starUnit": "성급",
        "hotel.search": "호텔 검색", "hotel.noHotels": "이 도시에는 호텔이 없습니다",
        "hotel.hint": "도시 코드를 입력하세요", "hotel.rooms": "객실",
        "hotel.breakfast": "조식 포함", "hotel.inventory": "남음:",
        "hotel.book": "예약", "hotel.checkIn": "체크인", "hotel.checkOut": "체크아웃",
        "order.product": "상품", "payment.title": "결제",
        "payment.channels": "결제 수단 선택", "payment.goPay": "지금 결제",
        "payment.checkoutHint": "브라우저에서 결제를 완료하세요. 이 페이지는 자동으로 새로고침됩니다",
        "payment.success": "결제 성공",
        "payment.timeout": "결제 결과를 확인할 수 없습니다. 나중에 주문을 확인하세요",
    },
    "ru": {
        "flight.title": "Авиабилеты", "flight.from": "Откуда", "flight.to": "Куда",
        "flight.date": "Дата вылета", "flight.cabin": "Класс", "flight.cabin.all": "Все классы",
        "flight.cabin.economy": "Эконом", "flight.cabin.business": "Бизнес", "flight.cabin.first": "Первый класс",
        "flight.search": "Найти авиабилеты", "flight.seatsLeft": "Осталось мест:",
        "flight.noFlights": "Нет рейсов по этому маршруту",
        "flight.hint": "Введите коды городов вылета и прибытия",
        "hotel.title": "Отели", "hotel.city": "Код города", "hotel.star": "Звёзды",
        "hotel.allStars": "Любая категория", "hotel.starUnit": "★",
        "hotel.search": "Найти отели", "hotel.noHotels": "В этом городе нет отелей",
        "hotel.hint": "Введите код города", "hotel.rooms": "Номера",
        "hotel.breakfast": "Завтрак включён", "hotel.inventory": "Осталось:",
        "hotel.book": "Забронировать", "hotel.checkIn": "Заезд", "hotel.checkOut": "Выезд",
        "order.product": "Товар", "payment.title": "Оплата",
        "payment.channels": "Выберите способ оплаты", "payment.goPay": "Оплатить",
        "payment.checkoutHint": "Завершите оплату в браузере, страница обновится автоматически",
        "payment.success": "Оплата прошла успешно",
        "payment.timeout": "Результат оплаты не подтверждён. Проверьте заказ позже",
    },
    "de": {
        "flight.title": "Flüge", "flight.from": "Von", "flight.to": "Nach",
        "flight.date": "Abflugdatum", "flight.cabin": "Klasse", "flight.cabin.all": "Alle Klassen",
        "flight.cabin.economy": "Economy", "flight.cabin.business": "Business", "flight.cabin.first": "First",
        "flight.search": "Flüge suchen", "flight.seatsLeft": "Freie Plätze:",
        "flight.noFlights": "Keine Flüge auf dieser Strecke",
        "flight.hint": "Codes von Abflug- und Ankunftsstadt eingeben",
        "hotel.title": "Hotels", "hotel.city": "Städtecode", "hotel.star": "Sternebewertung",
        "hotel.allStars": "Alle Sterne", "hotel.starUnit": "-Sterne",
        "hotel.search": "Hotels suchen", "hotel.noHotels": "Keine Hotels in dieser Stadt",
        "hotel.hint": "Städtecode eingeben", "hotel.rooms": "Zimmer",
        "hotel.breakfast": "Frühstück inklusive", "hotel.inventory": "Übrig:",
        "hotel.book": "Buchen", "hotel.checkIn": "Check-in", "hotel.checkOut": "Check-out",
        "order.product": "Produkt", "payment.title": "Zahlung",
        "payment.channels": "Zahlungsart wählen", "payment.goPay": "Jetzt bezahlen",
        "payment.checkoutHint": "Schließen Sie die Zahlung im Browser ab, diese Seite aktualisiert sich automatisch",
        "payment.success": "Zahlung erfolgreich",
        "payment.timeout": "Zahlungsergebnis nicht erkannt. Bestellung später prüfen",
    },
    "fr": {
        "flight.title": "Vols", "flight.from": "De", "flight.to": "À",
        "flight.date": "Date de départ", "flight.cabin": "Classe", "flight.cabin.all": "Toutes les classes",
        "flight.cabin.economy": "Économique", "flight.cabin.business": "Affaires", "flight.cabin.first": "Première",
        "flight.search": "Rechercher des vols", "flight.seatsLeft": "Places restantes :",
        "flight.noFlights": "Aucun vol sur cette liaison",
        "flight.hint": "Saisissez les codes ville de départ et d'arrivée",
        "hotel.title": "Hôtels", "hotel.city": "Code ville", "hotel.star": "Note en étoiles",
        "hotel.allStars": "Toutes les étoiles", "hotel.starUnit": "★",
        "hotel.search": "Rechercher des hôtels", "hotel.noHotels": "Aucun hôtel dans cette ville",
        "hotel.hint": "Saisissez un code ville", "hotel.rooms": "Chambres",
        "hotel.breakfast": "Petit-déjeuner inclus", "hotel.inventory": "Restant :",
        "hotel.book": "Réserver", "hotel.checkIn": "Arrivée", "hotel.checkOut": "Départ",
        "order.product": "Produit", "payment.title": "Paiement",
        "payment.channels": "Choisissez un moyen de paiement", "payment.goPay": "Payer",
        "payment.checkoutHint": "Terminez le paiement dans le navigateur, cette page se rafraîchira automatiquement",
        "payment.success": "Paiement réussi",
        "payment.timeout": "Résultat du paiement non détecté. Vérifiez la commande plus tard",
    },
    "es": {
        "flight.title": "Vuelos", "flight.from": "Desde", "flight.to": "Hacia",
        "flight.date": "Fecha de salida", "flight.cabin": "Clase", "flight.cabin.all": "Todas las clases",
        "flight.cabin.economy": "Económica", "flight.cabin.business": "Business", "flight.cabin.first": "Primera",
        "flight.search": "Buscar vuelos", "flight.seatsLeft": "Asientos disponibles:",
        "flight.noFlights": "No hay vuelos en esta ruta",
        "flight.hint": "Introduce los códigos de ciudad de salida y llegada",
        "hotel.title": "Hoteles", "hotel.city": "Código de ciudad", "hotel.star": "Estrellas",
        "hotel.allStars": "Todas las estrellas", "hotel.starUnit": "★",
        "hotel.search": "Buscar hoteles", "hotel.noHotels": "No hay hoteles en esta ciudad",
        "hotel.hint": "Introduce un código de ciudad", "hotel.rooms": "Habitaciones",
        "hotel.breakfast": "Desayuno incluido", "hotel.inventory": "Quedan:",
        "hotel.book": "Reservar", "hotel.checkIn": "Entrada", "hotel.checkOut": "Salida",
        "order.product": "Producto", "payment.title": "Pago",
        "payment.channels": "Elige un medio de pago", "payment.goPay": "Pagar ahora",
        "payment.checkoutHint": "Completa el pago en el navegador; esta página se actualizará automáticamente",
        "payment.success": "Pago realizado",
        "payment.timeout": "No se detectó el resultado del pago. Revisa el pedido más tarde",
    },
    "pt": {
        "flight.title": "Voos", "flight.from": "De", "flight.to": "Para",
        "flight.date": "Data de partida", "flight.cabin": "Classe", "flight.cabin.all": "Todas as classes",
        "flight.cabin.economy": "Econômica", "flight.cabin.business": "Executiva", "flight.cabin.first": "Primeira",
        "flight.search": "Pesquisar voos", "flight.seatsLeft": "Assentos restantes:",
        "flight.noFlights": "Não há voos nesta rota",
        "flight.hint": "Digite os códigos das cidades de origem e destino",
        "hotel.title": "Hotéis", "hotel.city": "Código da cidade", "hotel.star": "Estrelas",
        "hotel.allStars": "Todas as estrelas", "hotel.starUnit": "★",
        "hotel.search": "Pesquisar hotéis", "hotel.noHotels": "Não há hotéis nesta cidade",
        "hotel.hint": "Digite um código de cidade", "hotel.rooms": "Quartos",
        "hotel.breakfast": "Café da manhã incluído", "hotel.inventory": "Restam:",
        "hotel.book": "Reservar", "hotel.checkIn": "Check-in", "hotel.checkOut": "Check-out",
        "order.product": "Produto", "payment.title": "Pagamento",
        "payment.channels": "Escolha um meio de pagamento", "payment.goPay": "Pagar agora",
        "payment.checkoutHint": "Conclua o pagamento no navegador; esta página será atualizada automaticamente",
        "payment.success": "Pagamento realizado",
        "payment.timeout": "Resultado do pagamento não detectado. Verifique o pedido mais tarde",
    },
    "hi": {
        "flight.title": "फ़्लाइटें", "flight.from": "कहाँ से", "flight.to": "कहाँ तक",
        "flight.date": "प्रस्थान तिथि", "flight.cabin": "क्लास", "flight.cabin.all": "सभी क्लास",
        "flight.cabin.economy": "इकोनॉमी", "flight.cabin.business": "बिज़नेस", "flight.cabin.first": "फ़र्स्ट",
        "flight.search": "फ़्लाइट खोजें", "flight.seatsLeft": "शेष सीटें:",
        "flight.noFlights": "इस मार्ग पर कोई फ़्लाइट नहीं है",
        "flight.hint": "प्रस्थान और आगमन शहर कोड दर्ज करें",
        "hotel.title": "होटल", "hotel.city": "शहर कोड", "hotel.star": "स्टार रेटिंग",
        "hotel.allStars": "सभी स्टार", "hotel.starUnit": "★",
        "hotel.search": "होटल खोजें", "hotel.noHotels": "इस शहर में कोई होटल नहीं है",
        "hotel.hint": "शहर कोड दर्ज करें", "hotel.rooms": "कमरे",
        "hotel.breakfast": "नाश्ता शामिल", "hotel.inventory": "बचे हुए:",
        "hotel.book": "बुक करें", "hotel.checkIn": "चेक-इन", "hotel.checkOut": "चेक-आउट",
        "order.product": "उत्पाद", "payment.title": "भुगतान",
        "payment.channels": "भुगतान का तरीका चुनें", "payment.goPay": "अभी भुगतान करें",
        "payment.checkoutHint": "ब्राउज़र में भुगतान पूरा करें, यह पृष्ठ अपने आप रीफ़्रेश हो जाएगा",
        "payment.success": "भुगतान सफल",
        "payment.timeout": "भुगतान परिणाम पता नहीं चला। बाद में ऑर्डर जाँचें",
    },
    "ar": {
        "flight.title": "رحلات الطيران", "flight.from": "من", "flight.to": "إلى",
        "flight.date": "تاريخ المغادرة", "flight.cabin": "الدرجة", "flight.cabin.all": "جميع الدرجات",
        "flight.cabin.economy": "الاقتصادية", "flight.cabin.business": "درجة رجال الأعمال", "flight.cabin.first": "الدرجة الأولى",
        "flight.search": "ابحث عن رحلات", "flight.seatsLeft": "المقاعد المتبقية:",
        "flight.noFlights": "لا توجد رحلات على هذا المسار",
        "flight.hint": "أدخل رمزي مدينتي المغادرة والوصول",
        "hotel.title": "الفنادق", "hotel.city": "رمز المدينة", "hotel.star": "تصنيف النجوم",
        "hotel.allStars": "جميع النجوم", "hotel.starUnit": "★",
        "hotel.search": "ابحث عن فنادق", "hotel.noHotels": "لا توجد فنادق في هذه المدينة",
        "hotel.hint": "أدخل رمز المدينة", "hotel.rooms": "الغرف",
        "hotel.breakfast": "الإفطار مشمول", "hotel.inventory": "المتبقي:",
        "hotel.book": "احجز", "hotel.checkIn": "تسجيل الوصول", "hotel.checkOut": "تسجيل المغادرة",
        "order.product": "المنتج", "payment.title": "الدفع",
        "payment.channels": "اختر وسيلة الدفع", "payment.goPay": "ادفع الآن",
        "payment.checkoutHint": "أكمل الدفع في المتصفح، سيتم تحديث هذه الصفحة تلقائيًا",
        "payment.success": "تم الدفع بنجاح",
        "payment.timeout": "لم يتم اكتشاف نتيجة الدفع. تحقق من الطلب لاحقًا",
    },
    "bn": {
        "flight.title": "ফ্লাইট", "flight.from": "থেকে", "flight.to": "পর্যন্ত",
        "flight.date": "প্রস্থানের তারিখ", "flight.cabin": "ক্লাস", "flight.cabin.all": "সব ক্লাস",
        "flight.cabin.economy": "ইকোনমি", "flight.cabin.business": "বিজনেস", "flight.cabin.first": "ফার্স্ট",
        "flight.search": "ফ্লাইট খুঁজুন", "flight.seatsLeft": "বাকি আসন:",
        "flight.noFlights": "এই রুটে কোনো ফ্লাইট নেই",
        "flight.hint": "যাত্রা ও গন্তব্য শহরের কোড লিখুন",
        "hotel.title": "হোটেল", "hotel.city": "শহরের কোড", "hotel.star": "স্টার রেটিং",
        "hotel.allStars": "সব স্টার", "hotel.starUnit": "★",
        "hotel.search": "হোটেল খুঁজুন", "hotel.noHotels": "এই শহরে কোনো হোটেল নেই",
        "hotel.hint": "শহরের কোড লিখুন", "hotel.rooms": "কক্ষ",
        "hotel.breakfast": "নাস্তা অন্তর্ভুক্ত", "hotel.inventory": "বাকি:",
        "hotel.book": "বুক করুন", "hotel.checkIn": "চেক-ইন", "hotel.checkOut": "চেক-আউট",
        "order.product": "পণ্য", "payment.title": "পেমেন্ট",
        "payment.channels": "পেমেন্ট মাধ্যম বেছে নিন", "payment.goPay": "এখনই পেমেন্ট করুন",
        "payment.checkoutHint": "ব্রাউজারে পেমেন্ট সম্পন্ন করুন, এই পৃষ্ঠাটি স্বয়ংক্রিয়ভাবে রিফ্রেশ হবে",
        "payment.success": "পেমেন্ট সফল",
        "payment.timeout": "পেমেন্টের ফলাফল শনাক্ত করা যায়নি। পরে অর্ডার দেখুন",
    },
    "id": {
        "flight.title": "Penerbangan", "flight.from": "Dari", "flight.to": "Ke",
        "flight.date": "Tanggal berangkat", "flight.cabin": "Kelas", "flight.cabin.all": "Semua kelas",
        "flight.cabin.economy": "Ekonomi", "flight.cabin.business": "Bisnis", "flight.cabin.first": "Kelas satu",
        "flight.search": "Cari penerbangan", "flight.seatsLeft": "Kursi tersisa:",
        "flight.noFlights": "Tidak ada penerbangan di rute ini",
        "flight.hint": "Masukkan kode kota keberangkatan dan kedatangan",
        "hotel.title": "Hotel", "hotel.city": "Kode kota", "hotel.star": "Bintang",
        "hotel.allStars": "Semua bintang", "hotel.starUnit": "★",
        "hotel.search": "Cari hotel", "hotel.noHotels": "Tidak ada hotel di kota ini",
        "hotel.hint": "Masukkan kode kota", "hotel.rooms": "Kamar",
        "hotel.breakfast": "Termasuk sarapan", "hotel.inventory": "Tersisa:",
        "hotel.book": "Pesan", "hotel.checkIn": "Check-in", "hotel.checkOut": "Check-out",
        "order.product": "Produk", "payment.title": "Pembayaran",
        "payment.channels": "Pilih metode pembayaran", "payment.goPay": "Bayar sekarang",
        "payment.checkoutHint": "Selesaikan pembayaran di browser, halaman ini akan diperbarui otomatis",
        "payment.success": "Pembayaran berhasil",
        "payment.timeout": "Hasil pembayaran tidak terdeteksi. Periksa pesanan nanti",
    },
}

with open(f"{ARB}/en.arb", encoding="utf-8") as f:
    EN_KEYS = {k for k in json.load(f) if not k.startswith("@")}
for lang in LANGS:
    with open(f"{ARB}/{lang}.arb", encoding="utf-8") as f:
        d = json.load(f)
    missing = EN_KEYS - {k for k in d if not k.startswith("@")}
    if lang in NEW:
        uncovered = missing - set(NEW[lang])
        assert not uncovered, f"{lang}: NEW misses {sorted(uncovered)}"
        d.update(NEW[lang])
    keys = [k for k in d if not k.startswith("@")]
    # canonical order: en.arb first
    with open(f"{ARB}/en.arb", encoding="utf-8") as f:
        en = json.load(f)
    order = [k for k in en if not k.startswith("@")]
    extras = [k for k in keys if k not in order]
    ordered = order + extras
    out = {"@@locale": lang}
    for k in ordered:
        out[k] = d[k]
        out[f"@{k}"] = {"description": k}
    with open(f"{ARB}/{lang}.arb", "w", encoding="utf-8") as f:
        json.dump(out, f, ensure_ascii=False, indent=2)
        f.write("\n")

# consistency check
base = None
for lang in LANGS:
    with open(f"{ARB}/{lang}.arb", encoding="utf-8") as f:
        d = json.load(f)
    ks = [k for k in d if not k.startswith("@")]
    assert d["@@locale"] == lang
    if base is None:
        base = ks
    else:
        assert ks == base, f"{lang} key mismatch"
    meta = {k for k in d if k.startswith("@") and not k.startswith("@@")}
    assert meta == {f"@{k}" for k in ks}, f"{lang} metadata mismatch"
    print(f"{lang}: {len(ks)} keys OK")
print("ALL 13 ARBs consistent")
