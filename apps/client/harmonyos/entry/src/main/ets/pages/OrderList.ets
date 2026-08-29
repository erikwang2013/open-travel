import router from '@ohos.router';
import { ApiClient } from '../services/ApiClient';

class OrderItem {
  id: number = 0;
  amount_cents: number = 0;
  status: number = 0;
  created_at: string = '';
  expire_at: string = '';
  title: string = '';
  depart_date: string = '';
  quantity: number = 0;

  constructor(raw: Record<string, Object>) {
    this.id = (raw['id'] as number) ?? 0;
    this.amount_cents = (raw['amount_cents'] as number) ?? 0;
    this.status = (raw['status'] as number) ?? 0;
    this.created_at = (raw['created_at'] as string) ?? '';
    this.expire_at = (raw['expire_at'] as string) ?? '';
    const snap = raw['snapshot'];
    if (snap !== null && snap !== undefined) {
      const s = snap as Record<string, Object>;
      this.title = (s['title'] as string) ?? '';
      this.depart_date = (s['depart_date'] as string) ?? '';
      this.quantity = (s['quantity'] as number) ?? 0;
    }
  }
}

function parseOrders(data: Object): OrderItem[] {
  if (data === null || data === undefined) {
    return [];
  }
  if (data instanceof Array) {
    return (data as Object[]).map((it: Object) => new OrderItem(it as Record<string, Object>));
  }
  const items = (data as Record<string, Object>)['items'];
  if (items instanceof Array) {
    return (items as Object[]).map((it: Object) => new OrderItem(it as Record<string, Object>));
  }
  return [];
}

@Entry
@Component
struct OrderList {
  @State orders: OrderItem[] = [];
  @State loading: boolean = true;
  @State failed: boolean = false;
  @State notLoggedIn: boolean = false;
  private firstShow: boolean = true;

  aboutToAppear(): void {
    if (!ApiClient.isLoggedIn()) {
      this.notLoggedIn = true;
      this.loading = false;
      return;
    }
    this.load();
  }

  onPageShow(): void {
    if (this.firstShow) {
      this.firstShow = false;
      return;
    }
    if (!this.notLoggedIn) {
      this.load();
    }
  }

  async load(): Promise<void> {
    this.loading = true;
    this.failed = false;
    try {
      const data = await ApiClient.get<Object>('/api/orders?page=1&page_size=20');
      this.orders = parseOrders(data);
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

  statusText(s: number): Resource {
    if (s === 1) {
      return $r('app.string.order_status_1');
    }
    if (s === 2) {
      return $r('app.string.order_status_2');
    }
    if (s === 3) {
      return $r('app.string.order_status_3');
    }
    if (s === 4) {
      return $r('app.string.order_status_4');
    }
    return $r('app.string.order_status_0');
  }

  statusColor(s: number): string {
    if (s === 1) {
      return '#34C759';
    }
    if (s === 2) {
      return '#0A59F7';
    }
    if (s === 3) {
      return '#8E8E93';
    }
    if (s === 4) {
      return '#999999';
    }
    return '#FF9500';
  }

  @Builder
  loginView() {
    Column() {
      Text($r('app.string.login_required'))
        .fontSize(14)
        .fontColor('#999999')
      Button($r('app.string.go_login'), { type: ButtonType.Normal })
        .fontSize(14)
        .fontColor('#0A59F7')
        .backgroundColor(Color.Transparent)
        .margin({ top: 8 })
        .onClick(() => {
          router.pushUrl({ url: 'pages/LoginPage' });
        })
    }
    .width('100%')
    .layoutWeight(1)
    .justifyContent(FlexAlign.Center)
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
        Text($r('app.string.order_empty'))
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
      Text($r('app.string.order_list_title'))
        .fontSize(22)
        .fontWeight(FontWeight.Bold)
        .width('100%')
        .padding(16)

      if (this.notLoggedIn) {
        this.loginView()
      } else if (this.orders.length === 0) {
        this.stateView()
      } else {
        List({ space: 12 }) {
          ForEach(this.orders, (item: OrderItem) => {
            ListItem() {
              Column() {
                Row() {
                  Text(item.title.length > 0 ? item.title : ('#' + item.id.toString()))
                    .fontSize(16)
                    .fontWeight(FontWeight.Bold)
                    .layoutWeight(1)
                  Text(this.statusText(item.status))
                    .fontSize(11)
                    .fontColor('#FFFFFF')
                    .padding({ left: 8, right: 8, top: 3, bottom: 3 })
                    .borderRadius(8)
                    .backgroundColor(this.statusColor(item.status))
                }
                .width('100%')

                Row() {
                  Text($r('app.string.order_amount'))
                    .fontSize(13)
                    .fontColor('#999999')
                  Text(this.priceText(item.amount_cents))
                    .fontSize(15)
                    .fontColor('#E84026')
                    .fontWeight(FontWeight.Medium)
                  Blank()
                  Text(item.created_at.length > 0 ? item.created_at.slice(0, 10) : '')
                    .fontSize(12)
                    .fontColor('#999999')
                }
                .width('100%')
                .margin({ top: 8 })
              }
              .alignItems(HorizontalAlign.Start)
              .width('100%')
              .padding(16)
              .backgroundColor('#FFFFFF')
              .borderRadius(12)
              .onClick(() => {
                const p: Record<string, Object> = { 'id': item.id };
                router.pushUrl({ url: 'pages/OrderDetail', params: p });
              })
            }
          }, (item: OrderItem) => item.id.toString())
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
