import router from '@ohos.router';

class Destination {
  id: number;
  name: string;
  country: string;
  description: string;
  priceCents: number;

  constructor(id: number, name: string, country: string, description: string, priceCents: number) {
    this.id = id;
    this.name = name;
    this.country = country;
    this.description = description;
    this.priceCents = priceCents;
  }
}

class DestinationDataSource implements IDataSource {
  private list: Destination[];
  private listeners: DataChangeListener[] = [];

  constructor(list: Destination[]) {
    this.list = list;
  }

  totalCount(): number {
    return this.list.length;
  }

  getData(index: number): Destination {
    return this.list[index];
  }

  registerDataChangeListener(listener: DataChangeListener): void {
    this.listeners.push(listener);
  }

  unregisterDataChangeListener(listener: DataChangeListener): void {
    const pos: number = this.listeners.indexOf(listener);
    if (pos >= 0) {
      this.listeners.splice(pos, 1);
    }
  }
}

function createDestinations(): Destination[] {
  return [
    new Destination(1, '三亚', '中国', '热带海滨度假胜地，椰风海韵', 128000),
    new Destination(2, '北京', '中国', '千年古都，故宫与长城', 98000),
    new Destination(3, '张家界', '中国', '奇峰三千，云海仙境', 118000),
    new Destination(4, '桂林', '中国', '山水甲天下，漓江画廊', 88000),
    new Destination(5, '杭州', '中国', '西湖十景，人间天堂', 76000),
    new Destination(6, '成都', '中国', '大熊猫故乡，川味美食', 69000)
  ];
}

@Entry
@Component
struct DestinationList {
  private dataSource: DestinationDataSource = new DestinationDataSource(createDestinations());

  build() {
    Column() {
      Text($r('app.string.destination_list_title'))
        .fontSize(22)
        .fontWeight(FontWeight.Bold)
        .width('100%')
        .padding(16)

      List({ space: 12 }) {
        LazyForEach(this.dataSource, (item: Destination) => {
          ListItem() {
            Row() {
              Column() {
                Text(item.name)
                  .fontSize(18)
                  .fontWeight(FontWeight.Bold)
                Text(item.country)
                  .fontSize(12)
                  .fontColor('#999999')
                  .margin({ top: 4 })
                Text(item.description)
                  .fontSize(14)
                  .fontColor('#666666')
                  .maxLines(1)
                  .textOverflow({ overflow: TextOverflow.Ellipsis })
                  .margin({ top: 6 })
              }
              .alignItems(HorizontalAlign.Start)
              .layoutWeight(1)

              Text('¥' + (item.priceCents / 100).toFixed(0))
                .fontSize(16)
                .fontColor('#E84026')
                .fontWeight(FontWeight.Medium)
            }
            .width('100%')
            .padding(16)
            .backgroundColor('#FFFFFF')
            .borderRadius(12)
            .onClick(() => {
              router.pushUrl({ url: 'pages/DestinationDetail' });
            })
          }
        }, (item: Destination) => item.id.toString())
      }
      .width('100%')
      .layoutWeight(1)
      .padding({ left: 16, right: 16 })
    }
    .width('100%')
    .height('100%')
    .backgroundColor('#F5F5F5')
  }
}
