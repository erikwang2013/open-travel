import router from '@ohos.router';
import { ApiClient } from '../services/ApiClient';

class LineRow {
  id: number = 0;
  title: string = '';
  destination_id: number = 0;
  days: number = 0;
  price_cents: number = 0;
  max_pax: number = 0;
  cover_url: string = '';

  constructor(raw: Record<string, Object>) {
    this.id = (raw['id'] as number) ?? 0;
    this.title = (raw['title'] as string) ?? '';
    this.destination_id = (raw['destination_id'] as number) ?? 0;
    this.days = (raw['days'] as number) ?? 0;
    this.price_cents = (raw['price_cents'] as number) ?? 0;
    this.max_pax = (raw['max_pax'] as number) ?? 0;
    this.cover_url = (raw['cover_url'] as string) ?? '';
  }
}

interface LineListParams {
  destinationId?: number;
}

function parseLines(data: Object): LineRow[] {
  if (!(data instanceof Array)) {
    return [];
  }
  return (data as Object[]).map((it: Object) => new LineRow(it as Record<string, Object>));
}

@Entry
@Component
struct LineList {
  @State lines: LineRow[] = [];
  @State loading: boolean = true;
  @State failed: boolean = false;
  private destinationId: number = 0;

  aboutToAppear(): void {
    const params = router.getParams() as LineListParams;
    this.destinationId = params?.destinationId ?? 0;
    this.load();
  }

  async load(): Promise<void> {
    this.loading = true;
    this.failed = false;
    try {
      const q = this.destinationId > 0 ? `destination_id=${this.destinationId}&` : '';
      const data = await ApiClient.get<Object>(`/api/lines?${q}lang=${ApiClient.lang()}`);
      this.lines = parseLines(data);
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

  dayText(days: number): string {
    if (ApiClient.lang() === 'zh') {
      return days + ' 天';
    }
    return days + ' days';
  }

  groupText(maxPax: number): string {
    if (ApiClient.lang() === 'zh') {
      return '成团 ' + maxPax + ' 人';
    }
    return 'Group ' + maxPax;
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
        Text($r('app.string.line_empty'))
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
      Text($r('app.string.line_list_title'))
        .fontSize(22)
        .fontWeight(FontWeight.Bold)
        .width('100%')
        .padding(16)

      if (this.lines.length === 0) {
        this.stateView()
      } else {
        List({ space: 12 }) {
          ForEach(this.lines, (item: LineRow) => {
            ListItem() {
              Row() {
                Column() {
                  Text(item.title)
                    .fontSize(18)
                    .fontWeight(FontWeight.Bold)
                  Row() {
                    Text(this.dayText(item.days))
                      .fontSize(12)
                      .fontColor('#666666')
                    if (item.max_pax > 0) {
                      Text(this.groupText(item.max_pax))
                        .fontSize(12)
                        .fontColor('#666666')
                        .margin({ left: 10 })
                    }
                  }
                  .margin({ top: 6 })
                }
                .alignItems(HorizontalAlign.Start)
                .layoutWeight(1)

                Text(this.priceText(item.price_cents))
                  .fontSize(16)
                  .fontColor('#E84026')
                  .fontWeight(FontWeight.Medium)
              }
              .width('100%')
              .padding(16)
              .backgroundColor('#FFFFFF')
              .borderRadius(12)
              .onClick(() => {
                const p: Record<string, Object> = { 'id': item.id };
                router.pushUrl({ url: 'pages/LineDetail', params: p });
              })
            }
          }, (item: LineRow) => item.id.toString())
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
