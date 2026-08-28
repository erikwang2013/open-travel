import router from '@ohos.router';
import { ApiClient } from '../services/ApiClient';

class SearchItem {
  id: number = 0;
  item_type: string = '';
  name: string = '';
  price_cents: number = 0;
  description: string = '';

  constructor(raw: Record<string, Object>) {
    this.id = (raw['id'] as number) ?? 0;
    this.item_type = (raw['type'] as string) ?? '';
    this.name = (raw['name'] as string) ?? '';
    this.price_cents = (raw['price_cents'] as number) ?? 0;
    this.description = (raw['description'] as string) ?? '';
  }
}

interface SearchNavParams {
  q?: string;
}

function parseSearchResult(data: Object): SearchItem[] {
  if (data === null || data === undefined) {
    return [];
  }
  const items = (data as Record<string, Object>)['items'];
  if (!(items instanceof Array)) {
    return [];
  }
  return (items as Object[]).map((it: Object) => new SearchItem(it as Record<string, Object>));
}

@Entry
@Component
struct SearchPage {
  @State keyword: string = '';
  @State results: SearchItem[] = [];
  @State loading: boolean = false;
  @State failed: boolean = false;
  @State searched: boolean = false;

  aboutToAppear(): void {
    const params = router.getParams() as SearchNavParams;
    if (params && params.q && params.q.length > 0) {
      this.keyword = params.q;
      this.doSearch();
    }
  }

  async doSearch(): Promise<void> {
    if (this.keyword.length === 0) {
      return;
    }
    this.loading = true;
    this.failed = false;
    try {
      const url = `/api/search?q=${encodeURIComponent(this.keyword)}&lang=${ApiClient.lang()}&page=1`;
      const data = await ApiClient.get<Object>(url);
      this.results = parseSearchResult(data);
      this.searched = true;
    } catch (e) {
      this.failed = true;
    }
    this.loading = false;
  }

  priceText(cents: number): string {
    if (cents <= 0) {
      return '';
    }
    if (cents % 100 === 0) {
      return '¥' + (cents / 100).toFixed(0);
    }
    return '¥' + (cents / 100).toFixed(2);
  }

  typeText(t: string): Resource {
    if (t === 'attraction') {
      return $r('app.string.tag_attraction');
    }
    if (t === 'line') {
      return $r('app.string.tag_line');
    }
    if (t === 'hotel') {
      return $r('app.string.tag_hotel');
    }
    return $r('app.string.tag_destination');
  }

  typeColor(t: string): string {
    if (t === 'attraction') {
      return '#FF9500';
    }
    if (t === 'line') {
      return '#0A59F7';
    }
    if (t === 'hotel') {
      return '#34C759';
    }
    return '#8E8E93';
  }

  openItem(item: SearchItem): void {
    if (item.item_type === 'attraction') {
      const p: Record<string, Object> = { 'id': item.id };
      router.pushUrl({ url: 'pages/AttractionDetail', params: p });
      return;
    }
    router.pushUrl({ url: 'pages/DestinationList' });
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
            this.doSearch();
          })
      } else if (this.searched) {
        Text($r('app.string.search_empty'))
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
      Row() {
        TextInput({ placeholder: $r('app.string.search_hint'), text: this.keyword })
          .layoutWeight(1)
          .height(40)
          .onChange((v: string) => {
            this.keyword = v;
          })
        Button($r('app.string.search_button'), { type: ButtonType.Normal })
          .fontSize(15)
          .fontColor('#0A59F7')
          .backgroundColor(Color.Transparent)
          .onClick(() => {
            this.doSearch();
          })
      }
      .width('100%')
      .padding({ left: 16, right: 16, top: 12, bottom: 12 })
      .backgroundColor('#FFFFFF')

      if (!this.searched && !this.loading && !this.failed) {
        Text($r('app.string.search_placeholder'))
          .fontSize(14)
          .fontColor('#999999')
          .layoutWeight(1)
          .textAlign(TextAlign.Center)
      } else if (this.results.length === 0) {
        this.stateView()
      } else {
        List({ space: 12 }) {
          ForEach(this.results, (item: SearchItem) => {
            ListItem() {
              Column() {
                Row() {
                  Text(item.name)
                    .fontSize(17)
                    .fontWeight(FontWeight.Bold)
                    .layoutWeight(1)
                  Text(this.typeText(item.item_type))
                    .fontSize(11)
                    .fontColor('#FFFFFF')
                    .padding({ left: 8, right: 8, top: 3, bottom: 3 })
                    .borderRadius(8)
                    .backgroundColor(this.typeColor(item.item_type))
                }
                .width('100%')

                if (item.description.length > 0) {
                  Text(item.description)
                    .fontSize(13)
                    .fontColor('#666666')
                    .maxLines(1)
                    .textOverflow({ overflow: TextOverflow.Ellipsis })
                    .width('100%')
                    .margin({ top: 6 })
                }

                if (this.priceText(item.price_cents).length > 0) {
                  Text(this.priceText(item.price_cents))
                    .fontSize(15)
                    .fontColor('#E84026')
                    .fontWeight(FontWeight.Medium)
                    .margin({ top: 6 })
                }
              }
              .alignItems(HorizontalAlign.Start)
              .width('100%')
              .padding(16)
              .backgroundColor('#FFFFFF')
              .borderRadius(12)
              .onClick(() => {
                this.openItem(item);
              })
            }
          }, (item: SearchItem) => item.id.toString() + '_' + item.item_type)
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
