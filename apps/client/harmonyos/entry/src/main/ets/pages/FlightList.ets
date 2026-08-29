import router from '@ohos.router';
import { ApiClient } from '../services/ApiClient';

class FlightItem {
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

function parseFlights(data: Object): FlightItem[] {
  if (data === null || data === undefined) {
    return [];
  }
  if (data instanceof Array) {
    return (data as Object[]).map((it: Object) => new FlightItem(it as Record<string, Object>));
  }
  const items = (data as Record<string, Object>)['items'];
  if (items instanceof Array) {
    return (items as Object[]).map((it: Object) => new FlightItem(it as Record<string, Object>));
  }
  return [];
}

@Entry
@Component
struct FlightList {
  @State from: string = '';
  @State to: string = '';
  @State date: string = '';
  @State cabinIndex: number = 0;
  @State flights: FlightItem[] = [];
  @State loading: boolean = false;
  @State failed: boolean = false;
  @State searched: boolean = false;

  aboutToAppear(): void {
    this.date = this.todayStr();
  }

  todayStr(): string {
    const d = new Date();
    const m = (d.getMonth() + 1).toString().padStart(2, '0');
    const day = d.getDate().toString().padStart(2, '0');
    return d.getFullYear() + '-' + m + '-' + day;
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

  cabinOptions(): SelectOption[] {
    const opts: SelectOption[] = [{ value: $r('app.string.cabin_all') }];
    for (let c = 0; c < 3; c++) {
      opts.push({ value: this.cabinText(c) });
    }
    return opts;
  }

  async doSearch(): Promise<void> {
    const from = this.from.trim().toUpperCase();
    const to = this.to.trim().toUpperCase();
    if (from.length === 0 || to.length === 0) {
      return;
    }
    this.loading = true;
    this.failed = false;
    this.searched = true;
    try {
      let path = `/api/flights/search?from=${encodeURIComponent(from)}&to=${encodeURIComponent(to)}`;
      if (this.date.length > 0) {
        path += `&depart_date=${encodeURIComponent(this.date)}`;
      }
      if (this.cabinIndex > 0) {
        path += `&cabin=${this.cabinIndex - 1}`;
      }
      path += '&page=1&page_size=20';
      const data = await ApiClient.get<Object>(path);
      this.flights = parseFlights(data);
    } catch (e) {
      this.failed = true;
      this.flights = [];
    }
    this.loading = false;
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
        Text($r('app.string.flight_empty'))
          .fontSize(14)
          .fontColor('#999999')
      }
    }
    .width('100%')
    .layoutWeight(1)
    .justifyContent(FlexAlign.Center)
  }

  @Builder
  flightRow(f: FlightItem) {
    Row() {
      Column() {
        Text(f.from_code)
          .fontSize(17)
          .fontWeight(FontWeight.Bold)
        Text(f.depart_at.length > 0 ? f.depart_at.slice(0, 5) : '--:--')
          .fontSize(12)
          .fontColor('#999999')
          .margin({ top: 2 })
      }
      .alignItems(HorizontalAlign.Center)

      Column() {
        Text('—✈—')
          .fontSize(12)
          .fontColor('#BBBBBB')
        Text(f.airline + ' ' + f.flight_no)
          .fontSize(11)
          .fontColor('#999999')
          .margin({ top: 2 })
      }
      .alignItems(HorizontalAlign.Center)
      .layoutWeight(1)

      Column() {
        Text(f.to_code)
          .fontSize(17)
          .fontWeight(FontWeight.Bold)
        Text(f.arrive_at.length > 0 ? f.arrive_at.slice(0, 5) : '--:--')
          .fontSize(12)
          .fontColor('#999999')
          .margin({ top: 2 })
      }
      .alignItems(HorizontalAlign.Center)

      Column() {
        Text(this.priceText(f.price_cents))
          .fontSize(15)
          .fontColor('#E84026')
          .fontWeight(FontWeight.Medium)
        Text(this.cabinText(f.cabin))
          .fontSize(11)
          .fontColor('#999999')
          .margin({ top: 2 })
      }
      .alignItems(HorizontalAlign.End)
    }
    .width('100%')
    .padding(16)
    .backgroundColor('#FFFFFF')
    .borderRadius(12)
    .onClick(() => {
      const p: Record<string, Object> = { 'id': f.id };
      router.pushUrl({ url: 'pages/FlightDetail', params: p });
    })
  }

  build() {
    Column() {
      Text($r('app.string.flight_search_title'))
        .fontSize(22)
        .fontWeight(FontWeight.Bold)
        .width('100%')
        .padding(16)

      Column() {
        Row() {
          TextInput({ placeholder: $r('app.string.flight_from_hint'), text: this.from })
            .layoutWeight(1)
            .height(42)
            .onChange((v: string) => {
              this.from = v;
            })
          Text('⇄')
            .fontSize(18)
            .fontColor('#999999')
            .margin({ left: 8, right: 8 })
          TextInput({ placeholder: $r('app.string.flight_to_hint'), text: this.to })
            .layoutWeight(1)
            .height(42)
            .onChange((v: string) => {
              this.to = v;
            })
        }
        .width('100%')

        Row() {
          TextInput({ placeholder: $r('app.string.flight_date_hint'), text: this.date })
            .layoutWeight(1)
            .height(42)
            .onChange((v: string) => {
              this.date = v;
            })
          Select(this.cabinOptions())
            .selected(this.cabinIndex)
            .font({ size: 14 })
            .margin({ left: 8 })
            .onSelect((index: number) => {
              this.cabinIndex = index;
            })
        }
        .width('100%')
        .margin({ top: 10 })

        Button($r('app.string.flight_search_btn'))
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
      } else if (this.flights.length === 0) {
        this.stateView()
      } else {
        List({ space: 12 }) {
          ForEach(this.flights, (f: FlightItem) => {
            ListItem() {
              this.flightRow(f)
            }
          }, (f: FlightItem) => f.id.toString())
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
