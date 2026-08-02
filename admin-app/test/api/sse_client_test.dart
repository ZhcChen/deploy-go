import 'dart:convert';

import 'package:deploy_go_admin/api/sse_client.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('SSE 解析支持拆分 chunk、CRLF 和多行 data', () async {
    final bytes = <List<int>>[
      utf8.encode('id: 41\r\nevent: lo'),
      utf8.encode('g\r\ndata: {"line":\r\n'),
      utf8.encode('data: "ok"}\r\n\r\n'),
    ];

    final events = await parseSse(
      Stream<List<int>>.fromIterable(bytes),
    ).toList();

    expect(events, hasLength(1));
    expect(events.single.id, '41');
    expect(events.single.event, 'log');
    expect(events.single.data, '{"line":\n"ok"}');
  });

  test('SSE 解析在 CRLF 被分到不同 chunk 时仍及时派发', () async {
    final events = await parseSse(
      Stream<List<int>>.fromIterable(<List<int>>[
        utf8.encode('id: 1\r'),
        utf8.encode('\nevent: log\r'),
        utf8.encode('\ndata: {}\r'),
        utf8.encode('\n\r'),
        utf8.encode('\n'),
      ]),
    ).toList();

    expect(events, hasLength(1));
    expect(events.single.id, '1');
    expect(events.single.event, 'log');
  });

  test('SSE 解析忽略注释和不含 data 的事件', () async {
    final events = await parseSse(
      Stream<List<int>>.value(
        utf8.encode(': keepalive\n\nid: 2\n\nevent: terminal\ndata: {}\n\n'),
      ),
    ).toList();

    expect(events, hasLength(1));
    expect(events.single.event, 'terminal');
  });
}
