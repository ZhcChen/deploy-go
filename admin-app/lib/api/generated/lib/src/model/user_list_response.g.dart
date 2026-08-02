// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'user_list_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$UserListResponse extends UserListResponse {
  @override
  final BuiltList<UserResponse> items;
  @override
  final String? nextCursor;

  factory _$UserListResponse([
    void Function(UserListResponseBuilder)? updates,
  ]) => (UserListResponseBuilder()..update(updates))._build();

  _$UserListResponse._({required this.items, this.nextCursor}) : super._();
  @override
  UserListResponse rebuild(void Function(UserListResponseBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  UserListResponseBuilder toBuilder() =>
      UserListResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is UserListResponse &&
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
    return (newBuiltValueToStringHelper(r'UserListResponse')
          ..add('items', items)
          ..add('nextCursor', nextCursor))
        .toString();
  }
}

class UserListResponseBuilder
    implements Builder<UserListResponse, UserListResponseBuilder> {
  _$UserListResponse? _$v;

  ListBuilder<UserResponse>? _items;
  ListBuilder<UserResponse> get items =>
      _$this._items ??= ListBuilder<UserResponse>();
  set items(ListBuilder<UserResponse>? items) => _$this._items = items;

  String? _nextCursor;
  String? get nextCursor => _$this._nextCursor;
  set nextCursor(String? nextCursor) => _$this._nextCursor = nextCursor;

  UserListResponseBuilder() {
    UserListResponse._defaults(this);
  }

  UserListResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _items = $v.items.toBuilder();
      _nextCursor = $v.nextCursor;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(UserListResponse other) {
    _$v = other as _$UserListResponse;
  }

  @override
  void update(void Function(UserListResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  UserListResponse build() => _build();

  _$UserListResponse _build() {
    _$UserListResponse _$result;
    try {
      _$result =
          _$v ??
          _$UserListResponse._(items: items.build(), nextCursor: nextCursor);
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'items';
        items.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
          r'UserListResponse',
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
