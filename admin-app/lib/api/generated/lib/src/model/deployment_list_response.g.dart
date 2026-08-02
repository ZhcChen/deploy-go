// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'deployment_list_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$DeploymentListResponse extends DeploymentListResponse {
  @override
  final BuiltList<DeploymentResponse> items;
  @override
  final String? nextCursor;

  factory _$DeploymentListResponse(
          [void Function(DeploymentListResponseBuilder)? updates]) =>
      (DeploymentListResponseBuilder()..update(updates))._build();

  _$DeploymentListResponse._({required this.items, this.nextCursor})
      : super._();
  @override
  DeploymentListResponse rebuild(
          void Function(DeploymentListResponseBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  DeploymentListResponseBuilder toBuilder() =>
      DeploymentListResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is DeploymentListResponse &&
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
    return (newBuiltValueToStringHelper(r'DeploymentListResponse')
          ..add('items', items)
          ..add('nextCursor', nextCursor))
        .toString();
  }
}

class DeploymentListResponseBuilder
    implements Builder<DeploymentListResponse, DeploymentListResponseBuilder> {
  _$DeploymentListResponse? _$v;

  ListBuilder<DeploymentResponse>? _items;
  ListBuilder<DeploymentResponse> get items =>
      _$this._items ??= ListBuilder<DeploymentResponse>();
  set items(ListBuilder<DeploymentResponse>? items) => _$this._items = items;

  String? _nextCursor;
  String? get nextCursor => _$this._nextCursor;
  set nextCursor(String? nextCursor) => _$this._nextCursor = nextCursor;

  DeploymentListResponseBuilder() {
    DeploymentListResponse._defaults(this);
  }

  DeploymentListResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _items = $v.items.toBuilder();
      _nextCursor = $v.nextCursor;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(DeploymentListResponse other) {
    _$v = other as _$DeploymentListResponse;
  }

  @override
  void update(void Function(DeploymentListResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  DeploymentListResponse build() => _build();

  _$DeploymentListResponse _build() {
    _$DeploymentListResponse _$result;
    try {
      _$result = _$v ??
          _$DeploymentListResponse._(
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
            r'DeploymentListResponse', _$failedField, e.toString());
      }
      rethrow;
    }
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
