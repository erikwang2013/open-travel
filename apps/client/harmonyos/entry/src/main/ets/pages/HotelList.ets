import router from '@ohos.router';
import { ApiClient } from '../services/ApiClient';

class HotelItem {
  id: number = 0;
  name_en: string = '';
  name_zh: string = '';
  city_code: string = '';
  star: number = 0;
  cover_url: string = '';
  latitude: number = 0;
  longitude: number = 0;

  constructor(raw: Record<string, Object>) {
    this.id = (raw['id'] as number) ?? 0;
    this.name_en = (raw['name_en'] as string) ?? '';
    this.name_zh = (raw['name_zh'] as string) ?? '';
    this.city_code = (raw['city_code'] as string) ?? '';
    this.star = (raw['star'] as number) ?? 0;
    this.cover_url = (raw['cover_url'] as string) ?? '';
    this.latitude = (raw['latitude'] as number) ?? 0;
    this.longitude = (raw['longitude'] as number) ?? 0;
  }

  name(): string {
    const zh = this.name_zh.length > 0 ? this.name_zh : this.name_en;
    const en = this.name_en.length > 0 ? this.name_en : this.name_zh;
    return ApiClient.lang() === 'zh' ? zh : en;
  }
}

function parseHotels(data: Object): HotelItem[] {
  if (data === null || data === undefined) {
    return [];
  }
  if (data instanceof Array) {
    return (data as Object[]).map((it: Object) => new HotelItem(it as Record<string, Object>));
  }
  const items = (data as Record<string, Object>)['items'];
  if (items instanceof Array) {
    return (items as Object[]).map((it: Object) => new HotelItem(it as Record<string, Object>));
  }
  return [];
}

@Entry
@Component
struct HotelList {
  @State city: string = '';
  @State hotels: HotelItem[] = [];
  @State loading: boolean = false;
  @State failed: boolean = false;
  @State searched: boolean = false;

  async doSearch(): Promise<void> {
    const city = this.city.trim().toUpperCase();
    if (city.length === 0) {
      return;
    }
    this.loading = true;
    this.failed = false;
    this.searched = true;
    try {
      const path = `/api/hotels/search?city=${encodeURIComponent(city)}&page=1&page_size=20`;
      const data = await ApiClient.get<Object>(path);
      this.hotels = parseHotels(data);
    } catch (e) {
      this.failed = true;
      this.hotels = [];
    }
    this.loading = false;
  }

  starText(star: number): string {
    let s = '';
    for (let i = 0; i < star; i++) {
      s += '★';
    }
    return s;
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
      } else {
        Text($r('app.string.hotel_empty'))
          .fontSize(14)
          .fontColor('#999999')
      }
    }
    .width('100%')
    .layoutWeight(1)
    .justifyContent(FlexAlign.Center)
  }

  @Builder
  hotelRow(h: HotelItem) {
    Row() {
      if (h.cover_url.length > 0) {
        Image(h.cover_url)
          .width(72)
          .height(72)
          .borderRadius(8)
          .alt($r('app.media.icon'))
          .objectFit(ImageFit.Cover)
      }
      Column() {
        Text(h.name())
          .fontSize(16)
          .fontWeight(FontWeight.Bold)
          .maxLines(1)
          .textOverflow({ overflow: TextOverflow.Ellipsis })
        Row() {
          Text(this.starText(h.star))
            .fontSize(13)
            .fontColor('#FF9500')
          Text(h.city_code)
            .fontSize(12)
            .fontColor('#999999')
            .margin({ left: 8 })
        }
        .width('100%')
        .margin({ top: 6 })
      }
      .alignItems(HorizontalAlign.Start)
      .layoutWeight(1)
      .margin({ left: 12 })
    }
    .width('100%')
    .padding(16)
    .backgroundColor('#FFFFFF')
    .borderRadius(12)
    .onClick(() => {
      const p: Record<string, Object> = { 'id': h.id };
      router.pushUrl({ url: 'pages/HotelDetail', params: p });
    })
  }

  build() {
    Column() {
      Text($r('app.string.hotel_search_title'))
        .fontSize(22)
        .fontWeight(FontWeight.Bold)
        .width('100%')
        .padding(16)

      Column() {
        TextInput({ placeholder: $r('app.string.hotel_city_hint'), text: this.city })
          .width('100%')
          .height(42)
          .onChange((v: string) => {
            this.city = v;
          })
        Button($r('app.string.hotel_search_btn'))
          .width('100%')
          .height(42)
          .backgroundColor('#0A59F7')
          .margin({ top: 12 })
          .onClick(() => {
            this.doSearch();
          })
      }
      .width('100%')
      .padding(16)

      if (!this.searched) {
        Column() {
          Text($r('app.string.search_empty'))
            .fontSize(14)
            .fontColor('#999999')
        }
        .width('100%')
        .layoutWeight(1)
        .justifyContent(FlexAlign.Center)
      } else if (this.hotels.length === 0) {
        this.stateView()
      } else {
        List({ space: 12 }) {
          ForEach(this.hotels, (h: HotelItem) => {
            ListItem() {
              this.hotelRow(h)
            }
          }, (h: HotelItem) => h.id.toString())
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
