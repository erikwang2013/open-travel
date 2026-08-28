import 'package:flutter_test/flutter_test.dart';

import 'package:travel_admin/main.dart';

void main() {
  testWidgets('Admin skeleton renders', (WidgetTester tester) async {
    await tester.pumpWidget(const TravelAdminApp());
    expect(find.text('Open Travel 管理端'), findsOneWidget);
  });
}
