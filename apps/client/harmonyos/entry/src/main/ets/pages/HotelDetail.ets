import router from '@ohos.router';
import { ApiClient } from '../services/ApiClient';

class RoomItem {
  id: number = 0;
  room_type_en: string = '';
  room_type_zh: string = '';
  price_cents: number = 0;
  breakfast: number = 0;
  inventory: number = 0;

  constructor(raw: Record<string, Object>) {
    this.id = (raw['id'] as number) ?? 0;
    this.room_type_en = (raw['room_type_en'] as string) ?? '';
    this.room_type_zh = (raw['room_type_zh'] as string) ?? '';
    this.price_cents = (raw['price_cents'] as number) ?? 0;
    this.breakfast = (raw['breakfast'] as number) ?? 0;
    this.inventory = (raw['inventory'] as number) ?? 0;
  }

  name(): string {
    const zh = this.room_type_zh.length > 0 ? this.room_type_zh : this.room_type_en;
    const en = this.room_type_en.length > 0 ? this.room_type_en : this.room_type_zh;
    return ApiClient.lang() === 'zh' ? zh : en;
  }
}

class HotelDetailData {
  id: number = 0;
  name_en: string = '';
  name_zh: string = '';
  city_code: string = '';
  star: number = 0;
  cover_url: string = '';
  latitude: number = 0;
  longitude: number = 0;
  rooms: RoomItem[] = [];

  constructor(raw: Record<string, Object>) {
    this.id = (raw['id'] as number) ?? 0;
    this.name_en = (raw['name_en'] as string) ?? '';
    this.name_zh = (raw['name_zh'] as string) ?? '';
    this.city_code = (raw['city_code'] as string) ?? '';
    this.star = (raw['star'] as number) ?? 0;
    this.cover_url = (raw['cover_url'] as string) ?? '';
    this.latitude = (raw['latitude'] as number) ?? 0;
    this.longitude = (raw['longitude'] as number) ?? 0;
    const rs = raw['rooms'];
    if (rs instanceof Array) {
      this.rooms = (rs as Object[]).map((r: Object) => new RoomItem(r as Record<string, Object>));
    }
  }

  name(): string {
    const zh = this.name_zh.length > 0 ? this.name_zh : this.name_en;
    const en = this.name_en.length > 0 ? this.name_en : this.name_zh;
    return ApiClient.lang() === 'zh' ? zh : en;
  }
}

interface HotelDetailParams {
  id?: number;
}

@Entry
@Component
struct HotelDetail {
  @State detail: HotelDetailData | null = null;
  @State loading: boolean = true;
  @State failed: boolean = false;
  @State bookingId: number = -1;
  @State bookFailed: boolean = false;
  private hotelId: number = 0;

  aboutToAppear(): void {
    const params = router.getParams() as HotelDetailParams;
    this.hotelId = params?.id ?? 0;
    this.load();
  }

  async load(): Promise<void> {
    this.loading = true;
    this.failed = false;
    try {
      const raw = await ApiClient.get<Object>(`/api/hotels/${this.hotelId}`);
      this.detail = new HotelDetailData(raw as Record<string, Object>);
    } catch (e) {
      this.failed = true;
    }
    this.loading = false;
  }

  priceText(cents: number): string {
    if (cents % 100 === 0) {
      return '¥' + (cents / 100).toFixed(0);
    }
    return '¥' + (cents / 100).toFixed(2);
  }

  starText(star: number): string {
    let s = '';
    for (let i = 0; i < star; i++) {
      s += '★';
    }
    return s;
  }

  async doBook(roomId: number): Promise<void> {
    this.bookFailed = false;
    if (!ApiClient.isLoggedIn()) {
      router.pushUrl({ url: 'pages/LoginPage' });
      return;
    }
    this.bookingId = roomId;
    try {
      const body: Record<string, Object> = {
        'order_type': 3,
        'product_id': roomId,
        'quantity': 1
      };
      const order = await ApiClient.post<Record<string, Object>>('/api/orders', body, true);
      const orderId = (order['id'] as number) ?? 0;
      const p: Record<string, Object> = { 'id': orderId };
      router.pushUrl({ url: 'pages/PaymentPage', params: p });
    } catch (e) {
      this.bookFailed = true;
    }
    this.bookingId = -1;
  }

  @Builder
  failedView() {
    Column() {
      Text($r('app.string.load_failed'))
        .fontSize(14)
        .fontColor('#999999')
      Button($r('app.string.retry'), { type: ButtonType.Normal })
        .fontSize(14)
        .fontColor('#0A59F7')
        .backgroundColor(Color.Transparent)
        .margin({ top: 8 })
        .onClick(() => {
          this.load();
        })
    }
    .width('100%')
    .layoutWeight(1)
    .justifyContent(FlexAlign.Center)
  }

  @Builder
  roomRow(r: RoomItem) {
    Row() {
      Column() {
        Text(r.name())
          .fontSize(15)
          .fontWeight(FontWeight.Medium)
        Text(this.priceText(r.price_cents))
          .fontSize(15)
          .fontColor('#E84026')
          .margin({ top: 4 })
        Text((r.breakfast === 1 ? $r('app.string.breakfast_yes') : $r('app.string.breakfast_no')) as Resource)
          .fontSize(11)
          .fontColor('#999999')
          .margin({ top: 4 })
      }
      .alignItems(HorizontalAlign.Start)
      .layoutWeight(1)

      Button(this.bookingId === r.id ? $r('app.string.booking') : $r('app.string.book_room'))
        .height(36)
        .fontSize(13)
        .backgroundColor('#0A59F7')
        .enabled(this.bookingId < 0)
        .onClick(() => {
          this.doBook(r.id);
        })
    }
    .width('100%')
    .padding(16)
    .backgroundColor('#FFFFFF')
    .borderRadius(12)
    .margin({ top: 10 })
  }

  build() {
    Column() {
      Text($r('app.string.hotel_detail_title'))
        .fontSize(22)
        .fontWeight(FontWeight.Bold)
        .width('100%')
        .padding(16)

      if (this.loading) {
        Column() {
          LoadingProgress().width(48).height(48).color('#0A59F7')
        }
        .width('100%')
        .layoutWeight(1)
        .justifyContent(FlexAlign.Center)
      } else if (this.failed || !this.detail) {
        this.failedView()
      } else {
        Scroll() {
          Column() {
            if (this.detail.cover_url.length > 0) {
              Image(this.detail.cover_url)
                .width('100%')
                .height(180)
                .alt($r('app.media.icon'))
                .objectFit(ImageFit.Cover)
                .borderRadius(12)
            }

            Column() {
              Text(this.detail.name())
                .fontSize(20)
                .fontWeight(FontWeight.Bold)
                .width('100%')
              Row() {
                Text(this.starText(this.detail.star))
                  .fontSize(14)
                  .fontColor('#FF9500')
                Text(this.detail.city_code)
                  .fontSize(13)
                  .fontColor('#666666')
                  .margin({ left: 8 })
              }
              .width('100%')
              .margin({ top: 6 })
            }
            .alignItems(HorizontalAlign.Start)
            .width('100%')
            .padding(16)
            .backgroundColor('#FFFFFF')
            .borderRadius(12)
            .margin({ top: 12 })

            Text($r('app.string.rooms_title'))
              .fontSize(15)
              .fontWeight(FontWeight.Bold)
              .width('100%')
              .margin({ top: 16 })

            if (this.bookFailed) {
              Text($r('app.string.book_failed'))
                .fontSize(13)
                .fontColor('#E84026')
                .width('100%')
                .margin({ top: 8 })
            }

            if (this.detail.rooms.length === 0) {
              Text($r('app.string.rooms_empty'))
                .fontSize(13)
                .fontColor('#999999')
                .width('100%')
                .margin({ top: 8 })
            } else {
              ForEach(this.detail.rooms, (r: RoomItem) => {
                this.roomRow(r)
              }, (r: RoomItem) => r.id.toString())
            }
          }
          .alignItems(HorizontalAlign.Start)
          .width('100%')
          .padding(16)
        }
        .layoutWeight(1)
        .width('100%')
        .backgroundColor('#F5F5F5')
      }
    }
    .width('100%')
    .height('100%')
    .backgroundColor('#F5F5F5')
  }
}
