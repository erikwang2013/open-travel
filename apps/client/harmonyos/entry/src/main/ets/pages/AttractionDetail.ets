import router from '@ohos.router';
import { ApiClient } from '../services/ApiClient';

class AttractionDetailData {
  id: number;
  name: string;
  description: string;
  cover_url: string;
  price_cents: number;
  open_hours: string;
  rating_avg: number;

  constructor(id: number, name: string, description: string, cover_url: string,
              price_cents: number, open_hours: string, rating_avg: number) {
    this.id = id;
    this.name = name;
    this.description = description;
    this.cover_url = cover_url;
    this.price_cents = price_cents;
    this.open_hours = open_hours;
    this.rating_avg = rating_avg;
  }
}

interface DetailParams {
  id?: number;
}

@Entry
@Component
struct AttractionDetail {
  @State detail: AttractionDetailData | null = null;
  @State loading: boolean = true;
  @State failed: boolean = false;
  private attractionId: number = 0;

  aboutToAppear(): void {
    const params = router.getParams() as DetailParams;
    this.attractionId = params?.id ?? 0;
    this.load();
  }

  async load(): Promise<void> {
    this.loading = true;
    this.failed = false;
    try {
      this.detail = await ApiClient.get<AttractionDetailData>(
        `/api/booking/attractions/${this.attractionId}?lang=${ApiClient.lang()}`
      );
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

  @Builder
  failedView() {
    Column() {
      Text($r('app.string.attraction_not_found'))
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

  build() {
    Column() {
      Text($r('app.string.attraction_detail_title'))
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
                .height(220)
                .alt($r('app.media.icon'))
                .objectFit(ImageFit.Cover)
            }

            Column() {
              Row() {
                Text(this.detail.name)
                  .fontSize(20)
                  .fontWeight(FontWeight.Bold)
                  .layoutWeight(1)
                if (this.detail.rating_avg > 0) {
                  Text('★ ' + this.detail.rating_avg.toFixed(1))
                    .fontSize(14)
                    .fontColor('#FF9500')
                    .fontWeight(FontWeight.Medium)
                }
              }
              .width('100%')

              Row() {
                Text($r('app.string.ticket_price'))
                  .fontSize(14)
                  .fontColor('#999999')
                  .width(76)
                Text(this.detail.price_cents > 0 ? this.priceText(this.detail.price_cents) : $r('app.string.free_entry'))
                  .fontSize(14)
                  .fontColor('#E84026')
                  .fontWeight(FontWeight.Medium)
              }
              .width('100%')
              .margin({ top: 8 })

              if (this.detail.open_hours.length > 0) {
                Row() {
                  Text($r('app.string.open_hours'))
                    .fontSize(14)
                    .fontColor('#999999')
                    .width(76)
                  Text(this.detail.open_hours)
                    .fontSize(14)
                    .fontColor('#333333')
                }
                .width('100%')
                .margin({ top: 8 })
              }

              if (this.detail.description.length > 0) {
                Text(this.detail.description)
                  .fontSize(14)
                  .fontColor('#333333')
                  .lineHeight(22)
                  .width('100%')
                  .margin({ top: 12 })
              }
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
