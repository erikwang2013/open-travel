import router from '@ohos.router';
import { ApiClient } from '../services/ApiClient';

class ItineraryDay {
  day: number = 0;
  title: string = '';
  description: string = '';

  constructor(raw: Record<string, Object>) {
    this.day = (raw['day'] as number) ?? 0;
    this.title = (raw['title'] as string) ?? '';
    this.description = (raw['description'] as string) ?? '';
  }
}

class LineDetailData {
  id: number = 0;
  title: string = '';
  destination_id: number = 0;
  days: number = 0;
  price_cents: number = 0;
  max_pax: number = 0;
  cover_url: string = '';
  itinerary: ItineraryDay[] = [];

  constructor(raw: Record<string, Object>) {
    this.id = (raw['id'] as number) ?? 0;
    this.title = (raw['title'] as string) ?? '';
    this.destination_id = (raw['destination_id'] as number) ?? 0;
    this.days = (raw['days'] as number) ?? 0;
    this.price_cents = (raw['price_cents'] as number) ?? 0;
    this.max_pax = (raw['max_pax'] as number) ?? 0;
    this.cover_url = (raw['cover_url'] as string) ?? '';
    const it = raw['itinerary'];
    if (it instanceof Array) {
      this.itinerary = (it as Object[]).map((d: Object) => new ItineraryDay(d as Record<string, Object>));
    }
  }
}

class DepartureDate {
  id: number = 0;
  date: string = '';
  price_cents: number = 0;
  seats_left: number = 0;
  sold_out: boolean = false;

  constructor(raw: Record<string, Object>) {
    this.id = (raw['id'] as number) ?? 0;
    this.date = (raw['date'] as string) ?? '';
    this.price_cents = (raw['price_cents'] as number) ?? 0;
    this.seats_left = (raw['seats_left'] as number) ?? 0;
    this.sold_out = (raw['sold_out'] as boolean) ?? false;
  }
}

interface LineDetailParams {
  id?: number;
}

@Entry
@Component
struct LineDetail {
  @State detail: LineDetailData | null = null;
  @State dates: DepartureDate[] = [];
  @State selected: number = -1;
  @State loading: boolean = true;
  @State failed: boolean = false;
  @State booking: boolean = false;
  @State bookedId: number = 0;
  @State bookedAmount: number = 0;
  @State dateError: boolean = false;
  @State bookFailed: boolean = false;
  private lineId: number = 0;

  aboutToAppear(): void {
    const params = router.getParams() as LineDetailParams;
    this.lineId = params?.id ?? 0;
    this.loadAll();
  }

  async loadAll(): Promise<void> {
    this.loading = true;
    this.failed = false;
    try {
      const d = await ApiClient.get<Object>(`/api/lines/${this.lineId}?lang=${ApiClient.lang()}`);
      this.detail = new LineDetailData(d as Record<string, Object>);
      const ds = await ApiClient.get<Object>(`/api/lines/${this.lineId}/dates?lang=${ApiClient.lang()}`);
      if (ds instanceof Array) {
        this.dates = (ds as Object[]).map((x: Object) => new DepartureDate(x as Record<string, Object>));
      }
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

  dayText(day: number): string {
    if (ApiClient.lang() === 'zh') {
      return '第 ' + day + ' 天';
    }
    return 'Day ' + day;
  }

  async doBook(): Promise<void> {
    this.dateError = false;
    this.bookFailed = false;
    if (this.selected < 0) {
      this.dateError = true;
      return;
    }
    if (!ApiClient.isLoggedIn()) {
      router.pushUrl({ url: 'pages/LoginPage' });
      return;
    }
    const d = this.dates[this.selected];
    this.booking = true;
    try {
      const body: Record<string, Object> = {
        'order_type': 1,
        'product_id': this.lineId,
        'line_date_id': d.id,
        'quantity': 1
      };
      const order = await ApiClient.post<Record<string, Object>>('/api/orders', body, true);
      this.bookedId = (order['id'] as number) ?? 0;
      this.bookedAmount = (order['amount_cents'] as number) ?? 0;
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
          this.loadAll();
        })
    }
    .width('100%')
    .layoutWeight(1)
    .justifyContent(FlexAlign.Center)
  }

  build() {
    Column() {
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
                .height(200)
                .alt($r('app.media.icon'))
                .objectFit(ImageFit.Cover)
            }

            Column() {
              Text(this.detail.title)
                .fontSize(20)
                .fontWeight(FontWeight.Bold)
                .width('100%')

              Row() {
                Text(this.dayText(this.detail.days))
                  .fontSize(13)
                  .fontColor('#666666')
                Text($r('app.string.price'))
                  .fontSize(13)
                  .fontColor('#666666')
                  .margin({ left: 12 })
                Text(this.priceText(this.detail.price_cents))
                  .fontSize(15)
                  .fontColor('#E84026')
                  .fontWeight(FontWeight.Medium)
                Text($r('app.string.group_size'))
                  .fontSize(13)
                  .fontColor('#666666')
                  .margin({ left: 12 })
                Text(this.detail.max_pax.toString())
                  .fontSize(13)
                  .fontColor('#666666')
              }
              .width('100%')
              .margin({ top: 8 })

              if (this.detail.itinerary.length > 0) {
                Text($r('app.string.itinerary_title'))
                  .fontSize(16)
                  .fontWeight(FontWeight.Bold)
                  .width('100%')
                  .margin({ top: 16, bottom: 8 })

                ForEach(this.detail.itinerary, (item: ItineraryDay) => {
                  Column() {
                    Text(this.dayText(item.day) + '  ' + item.title)
                      .fontSize(14)
                      .fontWeight(FontWeight.Medium)
                      .fontColor('#0A59F7')
                      .width('100%')
                    if (item.description.length > 0) {
                      Text(item.description)
                        .fontSize(13)
                        .fontColor('#333333')
                        .lineHeight(20)
                        .width('100%')
                        .margin({ top: 4 })
                    }
                  }
                  .alignItems(HorizontalAlign.Start)
                  .width('100%')
                  .padding(12)
                  .backgroundColor('#FFFFFF')
                  .borderRadius(10)
                  .margin({ bottom: 8 })
                }, (item: ItineraryDay) => item.day.toString())
              }

              if (this.dates.length > 0) {
                Text($r('app.string.dates_title'))
                  .fontSize(16)
                  .fontWeight(FontWeight.Bold)
                  .width('100%')
                  .margin({ top: 12, bottom: 8 })

                List({ space: 10 }) {
                  ForEach(this.dates, (d: DepartureDate, index: number) => {
                    ListItem() {
                      Column() {
                        Text(d.date)
                          .fontSize(14)
                          .fontWeight(FontWeight.Medium)
                        Text(d.sold_out ? $r('app.string.sold_out') : this.priceText(d.price_cents))
                          .fontSize(12)
                          .fontColor(d.sold_out ? '#999999' : '#E84026')
                          .margin({ top: 4 })
                        if (!d.sold_out) {
                          Row() {
                            Text($r('app.string.seats_left'))
                              .fontSize(11)
                              .fontColor('#999999')
                            Text(d.seats_left.toString())
                              .fontSize(11)
                              .fontColor('#999999')
                          }
                          .margin({ top: 2 })
                        }
                      }
                      .width(96)
                      .padding(10)
                      .borderRadius(10)
                      .backgroundColor(this.selected === index ? '#0A59F7' : '#FFFFFF')
                      .border({
                        width: 1,
                        color: this.selected === index ? '#0A59F7' : '#E5E5E5'
                      })
                      .opacity(d.sold_out ? 0.45 : 1)
                      .onClick(() => {
                        if (!d.sold_out) {
                          this.selected = index;
                          this.dateError = false;
                          this.bookFailed = false;
                        }
                      })
                    }
                  }, (d: DepartureDate, index: number) => index.toString())
                }
                .width('100%')
                .height(104)
                .listDirection(Axis.Horizontal)
              }

              if (this.dateError) {
                Text($r('app.string.select_date_hint'))
                  .fontSize(13)
                  .fontColor('#E84026')
                  .margin({ top: 10 })
              }

              if (this.bookFailed) {
                Text($r('app.string.book_failed'))
                  .fontSize(13)
                  .fontColor('#E84026')
                  .margin({ top: 10 })
              }

              if (this.bookedId > 0) {
                Column() {
                  Text($r('app.string.order_placed'))
                    .fontSize(15)
                    .fontWeight(FontWeight.Bold)
                    .fontColor('#34C759')
                  Row() {
                    Text($r('app.string.order_no'))
                      .fontSize(13)
                      .fontColor('#666666')
                    Text('#' + this.bookedId.toString())
                      .fontSize(13)
                      .fontColor('#666666')
                  }
                  .margin({ top: 4 })
                  Row() {
                    Text($r('app.string.order_amount'))
                      .fontSize(13)
                      .fontColor('#666666')
                    Text(this.priceText(this.bookedAmount))
                      .fontSize(13)
                      .fontColor('#E84026')
                  }
                  .margin({ top: 2 })
                  Button($r('app.string.view_orders'), { type: ButtonType.Normal })
                    .fontSize(14)
                    .fontColor('#0A59F7')
                    .backgroundColor(Color.Transparent)
                    .margin({ top: 6 })
                    .onClick(() => {
                      router.pushUrl({ url: 'pages/OrderList' });
                    })
                }
                .width('100%')
                .padding(14)
                .backgroundColor('#FFFFFF')
                .borderRadius(10)
                .margin({ top: 12 })
              }

              Button(this.booking ? $r('app.string.booking') : $r('app.string.book_now'))
                .width('100%')
                .height(46)
                .backgroundColor('#0A59F7')
                .margin({ top: 16, bottom: 24 })
                .enabled(!this.booking)
                .onClick(() => {
                  this.doBook();
                })
            }
            .width('100%')
            .padding(16)
            .alignItems(HorizontalAlign.Start)
          }
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
