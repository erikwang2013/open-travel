import router from '@ohos.router';
import { ApiClient } from '../services/ApiClient';

class Attraction {
  id: number;
  name: string;
  cover_url: string;
  price_cents: number;
  open_hours: string;
  rating_avg: number;

  constructor(id: number, name: string, cover_url: string,
              price_cents: number, open_hours: string, rating_avg: number) {
    this.id = id;
    this.name = name;
    this.cover_url = cover_url;
    this.price_cents = price_cents;
    this.open_hours = open_hours;
    this.rating_avg = rating_avg;
  }
}

interface ListParams {
  destinationId?: number;
  name?: string;
}

interface DetailNavParams {
  id: number;
}

@Entry
@Component
struct AttractionList {
  @State attractions: Attraction[] = [];
  @State loading: boolean = true;
  @State failed: boolean = false;
  private destinationId: number = 0;
  private title: string = '';

  aboutToAppear(): void {
    const params = router.getParams() as ListParams;
    this.destinationId = params?.destinationId ?? 0;
    this.title = params?.name ?? '';
    this.load();
  }

  async load(): Promise<void> {
    this.loading = true;
    this.failed = false;
    try {
      const list = await ApiClient.get<Attraction[]>(
        `/api/booking/attractions?destination_id=${this.destinationId}&lang=${ApiClient.lang()}`
      );
      this.attractions = list ?? [];
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
  stateView() {
    Column() {
      if (this.loading) {
        LoadingProgress().width(48).height(48).color('#0A59F7')
      } else if (this.failed) {
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
      } else {
        Text($r('app.string.attraction_empty'))
          .fontSize(14)
          .fontColor('#999999')
      }
    }
    .width('100%')
    .layoutWeight(1)
    .justifyContent(FlexAlign.Center)
  }

  build() {
    Column() {
      Text(this.title.length > 0 ? this.title : $r('app.string.attraction_list_title'))
        .fontSize(22)
        .fontWeight(FontWeight.Bold)
        .width('100%')
        .padding(16)

      if (this.attractions.length === 0) {
        this.stateView()
      } else {
        List({ space: 12 }) {
          ForEach(this.attractions, (item: Attraction) => {
            ListItem() {
              Row() {
                Column() {
                  Text(item.name)
                    .fontSize(18)
                    .fontWeight(FontWeight.Bold)
                  if (item.rating_avg > 0) {
                    Text('★ ' + item.rating_avg.toFixed(1))
                      .fontSize(12)
                      .fontColor('#FF9500')
                      .margin({ top: 6 })
                  }
                }
                .alignItems(HorizontalAlign.Start)
                .layoutWeight(1)

                Text(item.price_cents > 0 ? this.priceText(item.price_cents) : $r('app.string.free_entry'))
                  .fontSize(16)
                  .fontColor('#E84026')
                  .fontWeight(FontWeight.Medium)
              }
              .width('100%')
              .padding(16)
              .backgroundColor('#FFFFFF')
              .borderRadius(12)
              .onClick(() => {
                const navParams: DetailNavParams = { id: item.id };
                router.pushUrl({
                  url: 'pages/AttractionDetail',
                  params: navParams
                });
              })
            }
          }, (item: Attraction) => item.id.toString())
        }
        .width('100%')
        .layoutWeight(1)
        .padding({ left: 16, right: 16 })
      }
    }
    .width('100%')
    .height('100%')
    .backgroundColor('#F5F5F5')
  }
}
