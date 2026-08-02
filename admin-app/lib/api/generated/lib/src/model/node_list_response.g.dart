// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'node_list_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$NodeListResponse extends NodeListResponse {
  @override
  final BuiltList<NodeResponse> items;
  @override
  final String? nextCursor;

  factory _$NodeListResponse(
          [void Function(NodeListResponseBuilder)? updates]) =>
      (NodeListResponseBuilder()..update(updates))._build();

  _$NodeListResponse._({required this.items, this.nextCursor}) : super._();
  @override
  NodeListResponse rebuild(void Function(NodeListResponseBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  NodeListResponseBuilder toBuilder() =>
      NodeListResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is NodeListResponse &&
        items == other.items &&
        nextCursor == other.nextCursor;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, items.hashCode);
    _$hash = $jc(_$hash, nextCursor.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'NodeListResponse')
          ..add('items', items)
          ..add('nextCursor', nextCursor))
        .toString();
  }
}

class NodeListResponseBuilder
    implements Builder<NodeListResponse, NodeListResponseBuilder> {
  _$NodeListResponse? _$v;

  ListBuilder<NodeResponse>? _items;
  ListBuilder<NodeResponse> get items =>
      _$this._items ??= ListBuilder<NodeResponse>();
  set items(ListBuilder<NodeResponse>? items) => _$this._items = items;

  String? _nextCursor;
  String? get nextCursor => _$this._nextCursor;
  set nextCursor(String? nextCursor) => _$this._nextCursor = nextCursor;

  NodeListResponseBuilder() {
    NodeListResponse._defaults(this);
  }

  NodeListResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _items = $v.items.toBuilder();
      _nextCursor = $v.nextCursor;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(NodeListResponse other) {
    _$v = other as _$NodeListResponse;
  }

  @override
  void update(void Function(NodeListResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  NodeListResponse build() => _build();

  _$NodeListResponse _build() {
    _$NodeListResponse _$result;
    try {
      _$result = _$v ??
          _$NodeListResponse._(
            items: items.build(),
            nextCursor: nextCursor,
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'items';
        items.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
            r'NodeListResponse', _$failedField, e.toString());
      }
      rethrow;
    }
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
