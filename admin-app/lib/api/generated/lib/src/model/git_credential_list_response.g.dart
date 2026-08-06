// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'git_credential_list_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$GitCredentialListResponse extends GitCredentialListResponse {
  @override
  final BuiltList<GitCredentialResponse> items;
  @override
  final String? nextCursor;

  factory _$GitCredentialListResponse([
    void Function(GitCredentialListResponseBuilder)? updates,
  ]) => (GitCredentialListResponseBuilder()..update(updates))._build();

  _$GitCredentialListResponse._({required this.items, this.nextCursor})
    : super._();
  @override
  GitCredentialListResponse rebuild(
    void Function(GitCredentialListResponseBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  GitCredentialListResponseBuilder toBuilder() =>
      GitCredentialListResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is GitCredentialListResponse &&
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
    return (newBuiltValueToStringHelper(r'GitCredentialListResponse')
          ..add('items', items)
          ..add('nextCursor', nextCursor))
        .toString();
  }
}

class GitCredentialListResponseBuilder
    implements
        Builder<GitCredentialListResponse, GitCredentialListResponseBuilder> {
  _$GitCredentialListResponse? _$v;

  ListBuilder<GitCredentialResponse>? _items;
  ListBuilder<GitCredentialResponse> get items =>
      _$this._items ??= ListBuilder<GitCredentialResponse>();
  set items(ListBuilder<GitCredentialResponse>? items) => _$this._items = items;

  String? _nextCursor;
  String? get nextCursor => _$this._nextCursor;
  set nextCursor(String? nextCursor) => _$this._nextCursor = nextCursor;

  GitCredentialListResponseBuilder() {
    GitCredentialListResponse._defaults(this);
  }

  GitCredentialListResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _items = $v.items.toBuilder();
      _nextCursor = $v.nextCursor;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(GitCredentialListResponse other) {
    _$v = other as _$GitCredentialListResponse;
  }

  @override
  void update(void Function(GitCredentialListResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  GitCredentialListResponse build() => _build();

  _$GitCredentialListResponse _build() {
    _$GitCredentialListResponse _$result;
    try {
      _$result =
          _$v ??
          _$GitCredentialListResponse._(
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
          r'GitCredentialListResponse',
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
