import router from '@ohos.router';
import { ApiClient } from '../services/ApiClient';

class FlightDetailData {
  id: number = 0;
  airline: string = '';
  flight_no: string = '';
  from_code: string = '';
  to_code: string = '';
  depart_at: string = '';
  arrive_at: string = '';
  cabin: number = 0;
  price_cents: number = 0;
  seats_left: number = 0;

  constructor(raw: Record<string, Object>) {
    this.id = (raw['id'] as number) ?? 0;
    this.airline = (raw['airline'] as string) ?? '';
    this.flight_no = (raw['flight_no'] as string) ?? '';
    this.from_code = (raw['from_code'] as string) ?? '';
    this.to_code = (raw['to_code'] as string) ?? '';
    this.depart_at = (raw['depart_at'] as string) ?? '';
    this.arrive_at = (raw['arrive_at'] as string) ?? '';
    this.cabin = (raw['cabin'] as number) ?? 0;
    this.price_cents = (raw['price_cents'] as number) ?? 0;
    this.seats_left = (raw['seats_left'] as number) ?? 0;
  }
}

interface FlightDetailParams {
  id?: number;
}

@Entry
@Component
struct FlightDetail {
  @State detail: FlightDetailData | null = null;
  @State loading: boolean = true;
  @State failed: boolean = false;
  @State booking: boolean = false;
  @State bookFailed: boolean = false;
  private flightId: number = 0;

  aboutToAppear(): void {
    const params = router.getParams() as FlightDetailParams;
    this.flightId = params?.id ?? 0;
    this.load();
  }

  async load(): Promise<void> {
    this.loading = true;
    this.failed = false;
    try {
      const raw = await ApiClient.get<Object>(`/api/flights/${this.flightId}`);
      this.detail = new FlightDetailData(raw as Record<string, Object>);
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

  cabinText(c: number): string {
    const zh: string = c === 1 ? '商务舱' : (c === 2 ? '头等舱' : '经济舱');
    const en: string = c === 1 ? 'Business' : (c === 2 ? 'First' : 'Economy');
    return ApiClient.lang() === 'zh' ? zh : en;
  }

  async doBook(): Promise<void> {
    this.bookFailed = false;
    if (!ApiClient.isLoggedIn()) {
      router.pushUrl({ url: 'pages/LoginPage' });
      return;
    }
    this.booking = true;
    try {
      const body: Record<string, Object> = {
        'order_type': 2,
        'product_id': this.flightId,
        'quantity': 1
      };
      const order = await ApiClient.post<Record<string, Object>>('/api/orders', body, true);
      const orderId = (order['id'] as number) ?? 0;
      const p: Record<string, Object> = { 'id': orderId };
      router.pushUrl({ url: 'pages/PaymentPage', params: p });
    } catch (e) {
      this.bookFailed = true;
    }
    this.booking = false;
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
  infoRow(label: Resource, value: string) {
    Row() {
      Text(label)
        .fontSize(14)
        .fontColor('#999999')
        .width(90)
      Text(value)
        .fontSize(14)
        .fontColor('#333333')
        .layoutWeight(1)
    }
    .width('100%')
    .margin({ top: 8 })
  }

  build() {
    Column() {
      Text($r('app.string.flight_detail_title'))
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
            Row() {
              Column() {
                Text(this.detail.from_code)
                  .fontSize(26)
                  .fontWeight(FontWeight.Bold)
                Text(this.detail.depart_at.length > 0 ? this.detail.depart_at.slice(0, 5) : '--:--')
                  .fontSize(13)
                  .fontColor('#999999')
                  .margin({ top: 4 })
              }
              .alignItems(HorizontalAlign.Center)

              Column() {
                Text('—✈—')
                  .fontSize(14)
                  .fontColor('#BBBBBB')
                Text(this.detail.airline + ' ' + this.detail.flight_no)
                  .fontSize(12)
                  .fontColor('#666666')
                  .margin({ top: 4 })
              }
              .alignItems(HorizontalAlign.Center)
              .layoutWeight(1)

              Column() {
                Text(this.detail.to_code)
                  .fontSize(26)
                  .fontWeight(FontWeight.Bold)
                Text(this.detail.arrive_at.length > 0 ? this.detail.arrive_at.slice(0, 5) : '--:--')
                  .fontSize(13)
                  .fontColor('#999999')
                  .margin({ top: 4 })
              }
              .alignItems(HorizontalAlign.Center)
            }
            .width('100%')
            .padding(16)
            .backgroundColor('#FFFFFF')
            .borderRadius(12)

            Column() {
              this.infoRow($r('app.string.order_amount'), this.priceText(this.detail.price_cents))
              this.infoRow($r('app.string.cabin_label'), this.cabinText(this.detail.cabin))
              this.infoRow($r('app.string.seats_left'), this.detail.seats_left.toString())
            }
            .alignItems(HorizontalAlign.Start)
            .width('100%')
            .padding(16)
            .backgroundColor('#FFFFFF')
            .borderRadius(12)
            .margin({ top: 12 })

            if (this.bookFailed) {
              Text($r('app.string.book_failed'))
                .fontSize(13)
                .fontColor('#E84026')
                .width('100%')
                .margin({ top: 12 })
            }

            Button(this.booking ? $r('app.string.booking') : $r('app.string.book_now'))
              .width('100%')
              .height(44)
              .backgroundColor('#0A59F7')
              .margin({ top: 16, bottom: 24 })
              .enabled(!this.booking)
              .onClick(() => {
                this.doBook();
              })
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
