import http from '@ohos.net.http';

export class ApiClient {
  static readonly BASE_URL: string = 'http://localhost:8002';

  static async get<T>(path: string): Promise<T> {
    const client = http.createHttp();
    try {
      const resp = await client.request(ApiClient.BASE_URL + path, {
        method: http.RequestMethod.GET,
        connectTimeout: 10000,
        readTimeout: 10000,
        header: { 'Content-Type': 'application/json' }
      });
      const code: number = resp.responseCode;
      if (code < 200 || code >= 300) {
        throw new Error('HTTP ' + code);
      }
      return JSON.parse(resp.result as string) as T;
    } finally {
      client.destroy();
    }
  }
}
