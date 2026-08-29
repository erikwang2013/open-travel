import promptAction from '@ohos.promptAction';
import router from '@ohos.router';
import preferences from '@ohos.data.preferences';
import common from '@ohos.app.ability.common';
import Want from '@ohos.app.ability.Want';
import { ApiClient } from '../services/ApiClient';

class UserProfile {
  user_id: number = 0;
  email: string = '';
  nickname: string = '';
  lang: string = '';

  constructor(raw: Record<string, Object>) {
    this.user_id = (raw['user_id'] as number) ?? 0;
    this.email = (raw['email'] as string) ?? '';
    this.nickname = (raw['nickname'] as string) ?? '';
    this.lang = (raw['lang'] as string) ?? '';
  }
}

class AddressItem {
  label: string = '';
  address: string = '';

  constructor(label: string, address: string) {
    this.label = label;
    this.address = address;
  }
}

const LANG_CODES: string[] = ['en', 'zh', 'ja', 'ko', 'ru', 'de', 'fr', 'es', 'pt', 'hi', 'ar', 'bn', 'id'];
const LANG_NAMES: string[] = ['English', '中文', '日本語', '한국어', 'Русский', 'Deutsch', 'Français',
  'Español', 'Português', 'हिन्दी', 'العربية', 'বাংলা', 'Bahasa Indonesia'];
const PREFS_NAME: string = 'travel_prefs';
const ADDR_KEY: string = 'address_book';

function langOptions(): SelectOption[] {
  const opts: SelectOption[] = [];
  for (const n of LANG_NAMES) {
    opts.push({ value: n });
  }
  return opts;
}

@Builder
function langSelect(selectedIndex: number, onChange: (index: number) => void) {
  Select(langOptions())
    .selected(selectedIndex)
    .font({ size: 15 })
    .onSelect((index: number) => {
      onChange(index);
    })
}

@Builder
function dialogActions(onCancel: () => void, onSave: () => void) {
  Row() {
    Button($r('app.string.cancel_btn'), { type: ButtonType.Normal })
      .fontSize(15)
      .fontColor('#999999')
      .backgroundColor(Color.Transparent)
      .layoutWeight(1)
      .onClick(onCancel)
    Button($r('app.string.profile_save'))
      .fontSize(15)
      .backgroundColor('#0A59F7')
      .layoutWeight(1)
      .onClick(onSave)
  }
  .width('100%')
  .margin({ top: 20 })
}

@CustomDialog
struct EditProfileDialog {
  controller: CustomDialogController;
  @State nickname: string = '';
  @State langIndex: number = 0;
  onSave: (nickname: string, lang: string) => void = () => {};

  build() {
    Column() {
      Text($r('app.string.profile_edit'))
        .fontSize(18)
        .fontWeight(FontWeight.Bold)
        .width('100%')

      TextInput({ placeholder: $r('app.string.profile_nickname'), text: this.nickname })
        .width('100%')
        .height(42)
        .margin({ top: 16 })
        .onChange((v: string) => {
          this.nickname = v;
        })

      Row() {
        Text($r('app.string.profile_lang'))
          .fontSize(15)
        Blank()
        langSelect(this.langIndex, (index: number) => {
          this.langIndex = index;
        })
      }
      .width('100%')
      .margin({ top: 16 })

      dialogActions(() => {
        this.controller.close();
      }, () => {
        this.onSave(this.nickname, LANG_CODES[this.langIndex]);
        this.controller.close();
      })
    }
    .width('100%')
    .padding(20)
  }
}

@CustomDialog
struct AddAddressDialog {
  controller: CustomDialogController;
  @State label: string = '';
  @State address: string = '';
  onAdd: (label: string, address: string) => void = () => {};

  build() {
    Column() {
      Text($r('app.string.profile_add_address'))
        .fontSize(18)
        .fontWeight(FontWeight.Bold)
        .width('100%')

      TextInput({ placeholder: $r('app.string.profile_nickname'), text: this.label })
        .width('100%')
        .height(42)
        .margin({ top: 16 })
        .onChange((v: string) => {
          this.label = v;
        })

      TextInput({ placeholder: $r('app.string.profile_address'), text: this.address })
        .width('100%')
        .height(42)
        .margin({ top: 12 })
        .onChange((v: string) => {
          this.address = v;
        })

      dialogActions(() => {
        this.controller.close();
      }, () => {
        this.onAdd(this.label, this.address);
        this.controller.close();
      })
    }
    .width('100%')
    .padding(20)
  }
}

@Entry
@Component
struct Profile {
  @State profile: UserProfile | null = null;
  @State addresses: AddressItem[] = [];
  @State loading: boolean = false;
  @State notLoggedIn: boolean = false;
  @State langIndex: number = 0;
  private firstShow: boolean = true;

  aboutToAppear(): void {
    if (!ApiClient.isLoggedIn()) {
      this.notLoggedIn = true;
      return;
    }
    this.refresh();
  }

  onPageShow(): void {
    if (this.firstShow) {
      this.firstShow = false;
      return;
    }
    this.refresh();
  }

  async refresh(): Promise<void> {
    if (!ApiClient.isLoggedIn()) {
      this.notLoggedIn = true;
      this.profile = null;
      return;
    }
    this.notLoggedIn = false;
    await this.loadProfile();
    await this.loadAddresses();
  }

  async loadProfile(): Promise<void> {
    this.loading = true;
    try {
      const raw = await ApiClient.get<Object>('/api/user/profile', true);
      this.profile = new UserProfile(raw as Record<string, Object>);
      const idx = LANG_CODES.indexOf(this.profile.lang);
      this.langIndex = idx >= 0 ? idx : 0;
    } catch (e) {
      promptAction.showToast({ message: $r('app.string.load_failed') });
    }
    this.loading = false;
  }

  async loadAddresses(): Promise<void> {
    const ctx = getContext(this) as common.UIAbilityContext;
    const store = await preferences.getPreferences(ctx, PREFS_NAME);
    const raw = await store.get(ADDR_KEY, '');
    if (typeof raw !== 'string' || raw.length === 0) {
      return;
    }
    const parsed = JSON.parse(raw) as Object;
    if (parsed instanceof Array) {
      this.addresses = (parsed as Object[]).map((it: Object) => {
        const r = it as Record<string, Object>;
        return new AddressItem((r['label'] as string) ?? '', (r['address'] as string) ?? '');
      });
    }
  }

  async saveAddresses(): Promise<void> {
    const ctx = getContext(this) as common.UIAbilityContext;
    const store = await preferences.getPreferences(ctx, PREFS_NAME);
    await store.put(ADDR_KEY, JSON.stringify(this.addresses));
    await store.flush();
  }

  setLang(code: string): void {
    AppStorage.SetOrCreate('app_lang', code);
    const ctx = getContext(this) as common.UIAbilityContext;
    ctx.getApplicationContext().setLanguage(code);
  }

  async saveProfile(nickname: string, lang: string): Promise<void> {
    try {
      const body: Record<string, Object> = { 'nickname': nickname, 'lang': lang };
      await ApiClient.put<Object>('/api/user/profile', body, true);
      this.setLang(lang);
      await this.loadProfile();
    } catch (e) {
      promptAction.showToast({ message: $r('app.string.save_failed') });
    }
  }

  openEdit(): void {
    const that = this;
    const nickname = this.profile !== null ? this.profile.nickname : '';
    const dlg = new CustomDialogController({
      builder: EditProfileDialog({
        nickname: nickname,
        langIndex: this.langIndex,
        onSave: (n: string, lang: string) => {
          that.saveProfile(n, lang);
        }
      }),
      autoCancel: true
    });
    dlg.open();
  }

  openAddAddress(): void {
    const that = this;
    const dlg = new CustomDialogController({
      builder: AddAddressDialog({
        onAdd: (label: string, address: string) => {
          that.addAddress(label, address);
        }
      }),
      autoCancel: true
    });
    dlg.open();
  }

  addAddress(label: string, address: string): void {
    if (address.trim().length === 0) {
      return;
    }
    this.addresses.push(new AddressItem(label.trim(), address.trim()));
    this.saveAddresses();
  }

  removeAddress(index: number): void {
    this.addresses.splice(index, 1);
    this.saveAddresses();
  }

  doLogout(): void {
    const that = this;
    const buttons: promptAction.Button[] = [
      { text: $r('app.string.cancel_btn'), color: '#999999' },
      { text: $r('app.string.logout'), color: '#E84026' }
    ];
    promptAction.showDialog({
      title: $r('app.string.logout'),
      message: $r('app.string.logout_confirm'),
      buttons: buttons
    }, () => {
      AppStorage.SetOrCreate('auth_token', '');
      AppStorage.SetOrCreate('user_email', '');
      that.profile = null;
      that.notLoggedIn = true;
    });
  }

  showSupport(): void {
    const ctx = getContext(this) as common.UIAbilityContext;
    const buttons: promptAction.Button[] = [
      { text: $r('app.string.close_btn'), color: '#999999' },
      { text: $r('app.string.support_call'), color: '#0A59F7' }
    ];
    promptAction.showDialog({
      title: $r('app.string.profile_support'),
      message: $r('app.string.support_contacts'),
      buttons: buttons
    }, () => {
      const want: Want = { action: 'ohos.want.action.dial', uri: 'tel:+864000000000' };
      ctx.startAbility(want).catch((e: Error) => {
        promptAction.showToast({ message: e.message });
      });
    });
  }

  @Builder
  loginView() {
    Column() {
      Text($r('app.string.profile_not_logged_in'))
        .fontSize(14)
        .fontColor('#999999')
      Button($r('app.string.profile_login'))
        .width(200)
        .height(42)
        .backgroundColor('#0A59F7')
        .margin({ top: 16 })
        .onClick(() => {
          router.pushUrl({ url: 'pages/LoginPage' });
        })
    }
    .width('100%')
    .layoutWeight(1)
    .justifyContent(FlexAlign.Center)
  }

  @Builder
  addressRow(item: AddressItem, index: number) {
    Row() {
      Column() {
        Text(item.label.length > 0 ? item.label : item.address)
          .fontSize(15)
          .fontWeight(FontWeight.Medium)
        Text(item.address)
          .fontSize(13)
          .fontColor('#666666')
          .margin({ top: 4 })
      }
      .alignItems(HorizontalAlign.Start)
      .layoutWeight(1)
      Button($r('app.string.profile_delete'), { type: ButtonType.Normal })
        .fontSize(13)
        .fontColor('#E84026')
        .backgroundColor(Color.Transparent)
        .onClick(() => {
          this.removeAddress(index);
        })
    }
    .width('100%')
    .padding(16)
    .backgroundColor('#FFFFFF')
    .borderRadius(10)
    .margin({ top: 10 })
  }

  build() {
    Column() {
      Text($r('app.string.profile_title'))
        .fontSize(22)
        .fontWeight(FontWeight.Bold)
        .width('100%')
        .padding(16)

      if (this.notLoggedIn) {
        this.loginView()
      } else if (this.loading) {
        Column() {
          LoadingProgress().width(48).height(48).color('#0A59F7')
        }
        .width('100%')
        .layoutWeight(1)
        .justifyContent(FlexAlign.Center)
      } else {
        Scroll() {
          Column() {
            Column() {
              Text('OT')
                .fontSize(28)
                .fontColor('#FFFFFF')
                .textAlign(TextAlign.Center)
                .width(72)
                .height(72)
                .borderRadius(36)
                .backgroundColor('#0A59F7')
              Text(this.profile !== null && this.profile.nickname.length > 0
                ? this.profile.nickname : (this.profile !== null ? this.profile.email : ''))
                .fontSize(18)
                .fontWeight(FontWeight.Bold)
                .margin({ top: 12 })
              Text(this.profile !== null ? this.profile.email : '')
                .fontSize(13)
                .fontColor('#999999')
                .margin({ top: 4 })

              Button($r('app.string.profile_edit'), { type: ButtonType.Normal })
                .fontSize(14)
                .fontColor('#0A59F7')
                .backgroundColor(Color.Transparent)
                .margin({ top: 12 })
                .onClick(() => {
                  this.openEdit();
                })
            }
            .width('100%')
            .padding({ top: 8, bottom: 16 })

            Row() {
              Text($r('app.string.profile_lang'))
                .fontSize(16)
              Blank()
              langSelect(this.langIndex, (index: number) => {
                this.langIndex = index;
                this.setLang(LANG_CODES[index]);
              })
            }
            .width('100%')
            .padding(16)
            .backgroundColor('#FFFFFF')
            .borderRadius(10)
            .margin({ top: 12 })

            Row() {
              Text($r('app.string.profile_address_book'))
                .fontSize(16)
                .layoutWeight(1)
              Button($r('app.string.profile_add_address'), { type: ButtonType.Normal })
                .fontSize(14)
                .fontColor('#0A59F7')
                .backgroundColor(Color.Transparent)
                .onClick(() => {
                  this.openAddAddress();
                })
            }
            .width('100%')
            .margin({ top: 20 })

            if (this.addresses.length === 0) {
              Text($r('app.string.address_empty'))
                .fontSize(13)
                .fontColor('#999999')
                .width('100%')
                .padding({ top: 12, bottom: 4 })
            } else {
              ForEach(this.addresses, (item: AddressItem, index: number) => {
                this.addressRow(item, index)
              }, (item: AddressItem) => item.address + '_' + this.addresses.indexOf(item).toString())
            }

            Row() {
              Text($r('app.string.profile_support'))
                .fontSize(16)
                .layoutWeight(1)
              Text('›')
                .fontSize(20)
                .fontColor('#999999')
            }
            .width('100%')
            .padding(16)
            .backgroundColor('#FFFFFF')
            .borderRadius(10)
            .margin({ top: 20 })
            .onClick(() => {
              this.showSupport();
            })

            Button($r('app.string.logout'))
              .width('100%')
              .height(44)
              .backgroundColor('#E84026')
              .margin({ top: 24, bottom: 24 })
              .onClick(() => {
                this.doLogout();
              })
          }
          .alignItems(HorizontalAlign.Start)
          .width('100%')
          .padding(16)
        }
        .layoutWeight(1)
        .width('100%')
      }
    }
    .width('100%')
    .height('100%')
    .backgroundColor('#F5F5F5')
  }
}
