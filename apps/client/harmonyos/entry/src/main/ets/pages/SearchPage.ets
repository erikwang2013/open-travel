import promptAction from '@ohos.promptAction';
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

class DestOption {
  id: number = 0;
  name: string = '';
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

function destName(raw: Record<string, Object>): string {
  const zh = raw['name_zh'];
  if (ApiClient.lang() === 'zh' && typeof zh === 'string' && (zh as string).length > 0) {
    return zh as string;
  }
  const en = raw['name_en'];
  if (typeof en === 'string' && (en as string).length > 0) {
    return en as string;
  }
  const name = raw['name'];
  if (typeof name === 'string' && (name as string).length > 0) {
    return name as string;
  }
  return 'ID ' + ((raw['id'] as number) ?? '');
}

function parseDestinations(data: Object): DestOption[] {
  if (data === null || data === undefined) {
    return [];
  }
  const list: Object[] = data instanceof Array ? (data as Object[]) : [];
  return list.map((it: Object) => {
    const r = it as Record<string, Object>;
    const d = new DestOption();
    d.id = (r['id'] as number) ?? 0;
    d.name = destName(r);
    return d;
  });
}

@Entry
@Component
struct SearchPage {
  @State keyword: string = '';
  @State results: SearchItem[] = [];
  @State destinations: DestOption[] = [];
  @State destIndex: number = 0;
  @State priceMin: string = '';
  @State priceMax: string = '';
  @State loading: boolean = false;
  @State failed: boolean = false;
  @State searched: boolean = false;

  aboutToAppear(): void {
    const params = router.getParams() as SearchNavParams;
    if (params && params.q && params.q.length > 0) {
      this.keyword = params.q;
      this.doSearch();
    }
    this.loadDestinations();
  }

  destOptions(): SelectOption[] {
    const opts: SelectOption[] = [{ value: $r('app.string.search_all') }];
    for (const d of this.destinations) {
      opts.push({ value: d.name });
    }
    return opts;
  }

  async loadDestinations(): Promise<void> {
    try {
      const data = await ApiClient.get<Object>('/api/booking/dates?region_id=1');
      this.destinations = parseDestinations(data);
    } catch (e) {
      // 目的地筛选拿不到时隐藏，搜索仍可用关键词
    }
  }

  async doSearch(): Promise<void> {
    if (this.keyword.length === 0 && this.destIndex === 0) {
      promptAction.showToast({ message: $r('app.string.search_need_keyword') });
      return;
    }
    this.loading = true;
    this.failed = false;
    try {
      let path = `/api/search?q=${encodeURIComponent(this.keyword)}&lang=${ApiClient.lang()}&page=1`;
      if (this.destIndex > 0) {
        const d = this.destinations[this.destIndex - 1];
        if (d !== undefined) {
          path += `&destination_id=${d.id}`;
        }
      }
      const min = parseInt(this.priceMin);
      const max = parseInt(this.priceMax);
      if (!isNaN(min) && min > 0) {
        path += `&price_min=${min}`;
      }
      if (!isNaN(max) && max > 0) {
        path += `&price_max=${max}`;
      }
      const data = await ApiClient.get<Object>(path);
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
      .padding({ left: 16, right: 16, top: 12, bottom: 8 })
      .backgroundColor('#FFFFFF')

      if (this.destinations.length > 0) {
        Row() {
          Text($r('app.string.search_dest_filter'))
            .fontSize(14)
            .fontColor('#666666')
          Blank()
          Select(this.destOptions())
            .selected(this.destIndex)
            .font({ size: 14 })
            .onSelect((index: number) => {
              this.destIndex = index;
            })
        }
        .width('100%')
        .padding({ left: 16, right: 16, top: 6, bottom: 6 })
        .backgroundColor('#FFFFFF')
      }

      Row() {
        TextInput({ placeholder: $r('app.string.search_min_price'), text: this.priceMin })
          .type(InputType.Number)
          .layoutWeight(1)
          .height(38)
          .onChange((v: string) => {
            this.priceMin = v;
          })
        Text('—')
          .fontSize(14)
          .fontColor('#999999')
          .margin({ left: 8, right: 8 })
        TextInput({ placeholder: $r('app.string.search_max_price'), text: this.priceMax })
          .type(InputType.Number)
          .layoutWeight(1)
          .height(38)
          .onChange((v: string) => {
            this.priceMax = v;
          })
      }
      .width('100%')
      .padding({ left: 16, right: 16, top: 8, bottom: 10 })
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
