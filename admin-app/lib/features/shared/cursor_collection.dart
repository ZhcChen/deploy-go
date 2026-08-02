import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../api/contracts.dart';

typedef CursorLoader<T> = Future<CursorPage<T>> Function(String? after);

class CursorCollectionState<T> {
  const CursorCollectionState({
    this.items = const [],
    this.nextCursor,
    this.loading = false,
    this.loadingMore = false,
    this.error,
    this.errorFromRefresh = false,
  });

  final List<T> items;
  final String? nextCursor;
  final bool loading;
  final bool loadingMore;
  final Object? error;
  final bool errorFromRefresh;

  bool get hasMore => nextCursor != null && nextCursor!.isNotEmpty;

  CursorCollectionState<T> copyWith({
    List<T>? items,
    String? nextCursor,
    bool clearCursor = false,
    bool? loading,
    bool? loadingMore,
    Object? error,
    bool clearError = false,
    bool? errorFromRefresh,
  }) => CursorCollectionState<T>(
    items: items ?? this.items,
    nextCursor: clearCursor ? null : nextCursor ?? this.nextCursor,
    loading: loading ?? this.loading,
    loadingMore: loadingMore ?? this.loadingMore,
    error: clearError ? null : error ?? this.error,
    errorFromRefresh: clearError
        ? false
        : errorFromRefresh ?? this.errorFromRefresh,
  );
}

class CursorCollectionController<T>
    extends StateNotifier<CursorCollectionState<T>> {
  CursorCollectionController(
    this._load,
    this._idOf, {
    this.clearItemsOnRefreshError = false,
  }) : super(CursorCollectionState<T>());

  final CursorLoader<T> _load;
  final String Function(T item) _idOf;
  final bool clearItemsOnRefreshError;
  int _generation = 0;

  Future<void> refresh() async {
    final generation = ++_generation;
    state = state.copyWith(loading: true, loadingMore: false, clearError: true);
    try {
      final page = await _load(null);
      if (!mounted || generation != _generation) return;
      state = CursorCollectionState<T>(
        items: _deduplicate(page.items),
        nextCursor: page.nextCursor,
      );
    } catch (error) {
      if (!mounted || generation != _generation) return;
      state = state.copyWith(
        items: clearItemsOnRefreshError ? const [] : null,
        clearCursor: clearItemsOnRefreshError,
        loading: false,
        error: error,
        errorFromRefresh: true,
      );
    }
  }

  Future<void> loadMore() async {
    if (!state.hasMore || state.loading || state.loadingMore) return;
    final generation = _generation;
    final cursor = state.nextCursor;
    state = state.copyWith(loadingMore: true, clearError: true);
    try {
      final page = await _load(cursor);
      if (!mounted || generation != _generation) return;
      state = CursorCollectionState<T>(
        items: _deduplicate(<T>[...state.items, ...page.items]),
        nextCursor: page.nextCursor,
      );
    } catch (error) {
      if (!mounted || generation != _generation) return;
      state = state.copyWith(loadingMore: false, error: error);
    }
  }

  List<T> _deduplicate(List<T> values) {
    final byId = <String, T>{};
    for (final value in values) {
      byId[_idOf(value)] = value;
    }
    return byId.values.toList(growable: false);
  }
}
