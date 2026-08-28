import http from '@ohos.net.http';
import i18n from '@ohos.i18n';

interface ApiEnvelope<T> {
  code: number;
  message: string;
  data: T;
}

export class ApiClient {
  static readonly BASE_URL: string = 'http://localhost:8082';
  static readonly API_VERSION: string = 'v1';

  static lang(): string {
    const sys = i18n.System.getSystemLanguage();
    if (sys.startsWith('zh')) {
      return 'zh';
    }
    return 'en';
  }

  static getToken(): string {
    const t = AppStorage.Get<string>('auth_token');
    return t === undefined ? '' : t;
  }

  static isLoggedIn(): boolean {
    return ApiClient.getToken().length > 0;
  }

  static async post<T>(path: string, body: Object, auth: boolean = false): Promise<T> {
    const client = http.createHttp();
    try {
      const header: Record<string, string> = {
        'Content-Type': 'application/json',
        'X-Api-Version': ApiClient.API_VERSION
      };
      if (auth) {
        const token = ApiClient.getToken();
        if (token.length === 0) {
          throw new Error('not logged in');
        }
        header['Authorization'] = 'Bearer ' + token;
      }
      const resp = await client.request(ApiClient.BASE_URL + path, {
        method: http.RequestMethod.POST,
        connectTimeout: 10000,
        readTimeout: 10000,
        header: header,
        extraData: JSON.stringify(body)
      });
      const code: number = resp.responseCode;
      if (code < 200 || code >= 300) {
        throw new Error('HTTP ' + code);
      }
      const env = JSON.parse(resp.result as string) as ApiEnvelope<T>;
      if (env.code !== 0) {
        throw new Error(env.message || ('API ' + env.code));
      }
      return env.data;
    } finally {
      client.destroy();
    }
  }

  static async get<T>(path: string): Promise<T> {
    const client = http.createHttp();
    try {
      const resp = await client.request(ApiClient.BASE_URL + path, {
        method: http.RequestMethod.GET,
        connectTimeout: 10000,
        readTimeout: 10000,
        header: {
          'Content-Type': 'application/json',
          'X-Api-Version': ApiClient.API_VERSION
        }
      });
      const code: number = resp.responseCode;
      if (code < 200 || code >= 300) {
        throw new Error('HTTP ' + code);
      }
      const env = JSON.parse(resp.result as string) as ApiEnvelope<T>;
      if (env.code !== 0) {
        throw new Error(env.message || ('API ' + env.code));
      }
      return env.data;
    } finally {
      client.destroy();
    }
  }
}
