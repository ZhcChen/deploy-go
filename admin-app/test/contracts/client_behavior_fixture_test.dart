import 'dart:convert';
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';

void main() {
  final fixture =
      jsonDecode(
            File('../test-fixtures/client-behavior.json').readAsStringSync(),
          )
          as Map<String, dynamic>;

  test('跨端 fixture 覆盖统一错误状态和 Request ID', () {
    final errors = fixture['errors'] as List<dynamic>;
    expect(
      errors.map((item) => (item as Map<String, dynamic>)['status']),
      <int>[401, 403, 409, 422, 500],
    );
    expect(
      errors.every(
        (item) =>
            ((item as Map<String, dynamic>)['request_id'] as String).isNotEmpty,
      ),
      isTrue,
    );
  });

  test('跨端 fixture 的 cursor 链按资源 ID 去重', () {
    final cursor = fixture['cursor'] as Map<String, dynamic>;
    final pages = cursor['pages'] as List<dynamic>;
    final items = pages
        .expand(
          (page) => (page as Map<String, dynamic>)['items'] as List<dynamic>,
        )
        .toSet()
        .toList();
    expect(items, cursor['expected_items']);
  });
}
