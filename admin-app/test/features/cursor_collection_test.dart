import 'dart:async';

import 'package:deploy_go_admin/api/contracts.dart';
import 'package:deploy_go_admin/features/shared/cursor_collection.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('cursor 追加按 ID 去重并在刷新时重置游标链', () async {
    final requested = <String?>[];
    final controller = CursorCollectionController<_Item>((after) async {
      requested.add(after);
      return switch (after) {
        null => const CursorPage(
          items: <_Item>[_Item('1'), _Item('2')],
          nextCursor: 'page-2',
        ),
        'page-2' => const CursorPage(items: <_Item>[_Item('2'), _Item('3')]),
        _ => const CursorPage(items: <_Item>[]),
      };
    }, (item) => item.id);

    await controller.refresh();
    await controller.loadMore();
    expect(controller.state.items.map((item) => item.id), <String>[
      '1',
      '2',
      '3',
    ]);
    expect(controller.state.hasMore, isFalse);

    await controller.refresh();
    expect(requested, <String?>[null, 'page-2', null]);
    expect(controller.state.items.map((item) => item.id), <String>['1', '2']);
  });

  test('追加失败保留已有数据和游标以便重试', () async {
    var fail = true;
    final controller = CursorCollectionController<_Item>((after) async {
      if (after == null) {
        return const CursorPage(items: <_Item>[_Item('1')], nextCursor: 'next');
      }
      if (fail) throw StateError('temporary');
      return const CursorPage(items: <_Item>[_Item('2')]);
    }, (item) => item.id);

    await controller.refresh();
    await controller.loadMore();
    expect(controller.state.items.single.id, '1');
    expect(controller.state.nextCursor, 'next');
    expect(controller.state.error, isA<StateError>());

    fail = false;
    await controller.loadMore();
    expect(controller.state.items.map((item) => item.id), <String>['1', '2']);
    expect(controller.state.error, isNull);
  });

  test('刷新失败保留已有数据和游标且可重试', () async {
    var refreshes = 0;
    final controller = CursorCollectionController<_Item>((after) async {
      if (after != null) {
        return const CursorPage(items: <_Item>[_Item('2')]);
      }
      refreshes += 1;
      if (refreshes == 2) throw StateError('refresh failed');
      return CursorPage(
        items: <_Item>[_Item(refreshes == 1 ? '1' : 'fresh')],
        nextCursor: 'next',
      );
    }, (item) => item.id);

    await controller.refresh();
    await controller.refresh();
    expect(controller.state.items.single.id, '1');
    expect(controller.state.nextCursor, 'next');
    expect(controller.state.errorFromRefresh, isTrue);

    await controller.refresh();
    expect(controller.state.items.single.id, 'fresh');
    expect(controller.state.error, isNull);
    expect(controller.state.errorFromRefresh, isFalse);
  });

  test('较早的刷新响应不会覆盖较新的列表', () async {
    final first = Completer<CursorPage<_Item>>();
    final second = Completer<CursorPage<_Item>>();
    var calls = 0;
    final controller = CursorCollectionController<_Item>((_) {
      calls += 1;
      return calls == 1 ? first.future : second.future;
    }, (item) => item.id);

    final firstRefresh = controller.refresh();
    final secondRefresh = controller.refresh();
    second.complete(const CursorPage(items: <_Item>[_Item('new')]));
    await secondRefresh;
    first.complete(const CursorPage(items: <_Item>[_Item('old')]));
    await firstRefresh;

    expect(controller.state.items.single.id, 'new');
  });

  test('刷新打断追加后不会残留加载状态', () async {
    final loadMore = Completer<CursorPage<_Item>>();
    var refreshes = 0;
    final controller = CursorCollectionController<_Item>((after) async {
      if (after != null) return loadMore.future;
      refreshes += 1;
      if (refreshes == 1) {
        return const CursorPage(items: <_Item>[_Item('1')], nextCursor: 'next');
      }
      throw StateError('refresh failed');
    }, (item) => item.id);

    await controller.refresh();
    final pendingLoadMore = controller.loadMore();
    await controller.refresh();

    expect(controller.state.loading, isFalse);
    expect(controller.state.loadingMore, isFalse);
    loadMore.complete(const CursorPage(items: <_Item>[_Item('2')]));
    await pendingLoadMore;
    expect(controller.state.items.map((item) => item.id), <String>['1']);
  });
}

class _Item {
  const _Item(this.id);
  final String id;
}
