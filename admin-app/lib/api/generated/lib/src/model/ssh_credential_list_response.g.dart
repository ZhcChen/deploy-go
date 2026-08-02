// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'ssh_credential_list_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$SshCredentialListResponse extends SshCredentialListResponse {
  @override
  final BuiltList<SshCredentialResponse> items;
  @override
  final String? nextCursor;

  factory _$SshCredentialListResponse([
    void Function(SshCredentialListResponseBuilder)? updates,
  ]) => (SshCredentialListResponseBuilder()..update(updates))._build();

  _$SshCredentialListResponse._({required this.items, this.nextCursor})
    : super._();
  @override
  SshCredentialListResponse rebuild(
    void Function(SshCredentialListResponseBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  SshCredentialListResponseBuilder toBuilder() =>
      SshCredentialListResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is SshCredentialListResponse &&
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
    return (newBuiltValueToStringHelper(r'SshCredentialListResponse')
          ..add('items', items)
          ..add('nextCursor', nextCursor))
        .toString();
  }
}

class SshCredentialListResponseBuilder
    implements
        Builder<SshCredentialListResponse, SshCredentialListResponseBuilder> {
  _$SshCredentialListResponse? _$v;

  ListBuilder<SshCredentialResponse>? _items;
  ListBuilder<SshCredentialResponse> get items =>
      _$this._items ??= ListBuilder<SshCredentialResponse>();
  set items(ListBuilder<SshCredentialResponse>? items) => _$this._items = items;

  String? _nextCursor;
  String? get nextCursor => _$this._nextCursor;
  set nextCursor(String? nextCursor) => _$this._nextCursor = nextCursor;

  SshCredentialListResponseBuilder() {
    SshCredentialListResponse._defaults(this);
  }

  SshCredentialListResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _items = $v.items.toBuilder();
      _nextCursor = $v.nextCursor;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(SshCredentialListResponse other) {
    _$v = other as _$SshCredentialListResponse;
  }

  @override
  void update(void Function(SshCredentialListResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  SshCredentialListResponse build() => _build();

  _$SshCredentialListResponse _build() {
    _$SshCredentialListResponse _$result;
    try {
      _$result =
          _$v ??
          _$SshCredentialListResponse._(
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
          r'SshCredentialListResponse',
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
