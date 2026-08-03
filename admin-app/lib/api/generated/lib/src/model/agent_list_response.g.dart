// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'agent_list_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$AgentListResponse extends AgentListResponse {
  @override
  final BuiltList<AgentResponse> items;
  @override
  final String? nextCursor;

  factory _$AgentListResponse([
    void Function(AgentListResponseBuilder)? updates,
  ]) => (AgentListResponseBuilder()..update(updates))._build();

  _$AgentListResponse._({required this.items, this.nextCursor}) : super._();
  @override
  AgentListResponse rebuild(void Function(AgentListResponseBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  AgentListResponseBuilder toBuilder() =>
      AgentListResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is AgentListResponse &&
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
    return (newBuiltValueToStringHelper(r'AgentListResponse')
          ..add('items', items)
          ..add('nextCursor', nextCursor))
        .toString();
  }
}

class AgentListResponseBuilder
    implements Builder<AgentListResponse, AgentListResponseBuilder> {
  _$AgentListResponse? _$v;

  ListBuilder<AgentResponse>? _items;
  ListBuilder<AgentResponse> get items =>
      _$this._items ??= ListBuilder<AgentResponse>();
  set items(ListBuilder<AgentResponse>? items) => _$this._items = items;

  String? _nextCursor;
  String? get nextCursor => _$this._nextCursor;
  set nextCursor(String? nextCursor) => _$this._nextCursor = nextCursor;

  AgentListResponseBuilder() {
    AgentListResponse._defaults(this);
  }

  AgentListResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _items = $v.items.toBuilder();
      _nextCursor = $v.nextCursor;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(AgentListResponse other) {
    _$v = other as _$AgentListResponse;
  }

  @override
  void update(void Function(AgentListResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  AgentListResponse build() => _build();

  _$AgentListResponse _build() {
    _$AgentListResponse _$result;
    try {
      _$result =
          _$v ??
          _$AgentListResponse._(items: items.build(), nextCursor: nextCursor);
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'items';
        items.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
          r'AgentListResponse',
          _$failedField,
          e.toString(),
        );
      }
      rethrow;
    }
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
