// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'deployment_target_list_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$DeploymentTargetListResponse extends DeploymentTargetListResponse {
  @override
  final BuiltList<DeploymentTargetResponse> items;
  @override
  final String? nextCursor;

  factory _$DeploymentTargetListResponse([
    void Function(DeploymentTargetListResponseBuilder)? updates,
  ]) => (DeploymentTargetListResponseBuilder()..update(updates))._build();

  _$DeploymentTargetListResponse._({required this.items, this.nextCursor})
    : super._();
  @override
  DeploymentTargetListResponse rebuild(
    void Function(DeploymentTargetListResponseBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  DeploymentTargetListResponseBuilder toBuilder() =>
      DeploymentTargetListResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is DeploymentTargetListResponse &&
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
    return (newBuiltValueToStringHelper(r'DeploymentTargetListResponse')
          ..add('items', items)
          ..add('nextCursor', nextCursor))
        .toString();
  }
}

class DeploymentTargetListResponseBuilder
    implements
        Builder<
          DeploymentTargetListResponse,
          DeploymentTargetListResponseBuilder
        > {
  _$DeploymentTargetListResponse? _$v;

  ListBuilder<DeploymentTargetResponse>? _items;
  ListBuilder<DeploymentTargetResponse> get items =>
      _$this._items ??= ListBuilder<DeploymentTargetResponse>();
  set items(ListBuilder<DeploymentTargetResponse>? items) =>
      _$this._items = items;

  String? _nextCursor;
  String? get nextCursor => _$this._nextCursor;
  set nextCursor(String? nextCursor) => _$this._nextCursor = nextCursor;

  DeploymentTargetListResponseBuilder() {
    DeploymentTargetListResponse._defaults(this);
  }

  DeploymentTargetListResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _items = $v.items.toBuilder();
      _nextCursor = $v.nextCursor;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(DeploymentTargetListResponse other) {
    _$v = other as _$DeploymentTargetListResponse;
  }

  @override
  void update(void Function(DeploymentTargetListResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  DeploymentTargetListResponse build() => _build();

  _$DeploymentTargetListResponse _build() {
    _$DeploymentTargetListResponse _$result;
    try {
      _$result =
          _$v ??
          _$DeploymentTargetListResponse._(
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
          r'DeploymentTargetListResponse',
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
