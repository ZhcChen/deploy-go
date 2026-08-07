// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'deployment_event_list_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$DeploymentEventListResponse extends DeploymentEventListResponse {
  @override
  final BuiltList<DeploymentEventResponse> items;
  @override
  final String? nextCursor;

  factory _$DeploymentEventListResponse([
    void Function(DeploymentEventListResponseBuilder)? updates,
  ]) => (DeploymentEventListResponseBuilder()..update(updates))._build();

  _$DeploymentEventListResponse._({required this.items, this.nextCursor})
    : super._();
  @override
  DeploymentEventListResponse rebuild(
    void Function(DeploymentEventListResponseBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  DeploymentEventListResponseBuilder toBuilder() =>
      DeploymentEventListResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is DeploymentEventListResponse &&
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
    return (newBuiltValueToStringHelper(r'DeploymentEventListResponse')
          ..add('items', items)
          ..add('nextCursor', nextCursor))
        .toString();
  }
}

class DeploymentEventListResponseBuilder
    implements
        Builder<
          DeploymentEventListResponse,
          DeploymentEventListResponseBuilder
        > {
  _$DeploymentEventListResponse? _$v;

  ListBuilder<DeploymentEventResponse>? _items;
  ListBuilder<DeploymentEventResponse> get items =>
      _$this._items ??= ListBuilder<DeploymentEventResponse>();
  set items(ListBuilder<DeploymentEventResponse>? items) =>
      _$this._items = items;

  String? _nextCursor;
  String? get nextCursor => _$this._nextCursor;
  set nextCursor(String? nextCursor) => _$this._nextCursor = nextCursor;

  DeploymentEventListResponseBuilder() {
    DeploymentEventListResponse._defaults(this);
  }

  DeploymentEventListResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _items = $v.items.toBuilder();
      _nextCursor = $v.nextCursor;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(DeploymentEventListResponse other) {
    _$v = other as _$DeploymentEventListResponse;
  }

  @override
  void update(void Function(DeploymentEventListResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  DeploymentEventListResponse build() => _build();

  _$DeploymentEventListResponse _build() {
    _$DeploymentEventListResponse _$result;
    try {
      _$result =
          _$v ??
          _$DeploymentEventListResponse._(
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
          r'DeploymentEventListResponse',
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
