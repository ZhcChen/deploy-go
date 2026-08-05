// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'agent_release_list_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$AgentReleaseListResponse extends AgentReleaseListResponse {
  @override
  final String? currentVersion;
  @override
  final BuiltList<AgentReleaseResponse> items;

  factory _$AgentReleaseListResponse([
    void Function(AgentReleaseListResponseBuilder)? updates,
  ]) => (AgentReleaseListResponseBuilder()..update(updates))._build();

  _$AgentReleaseListResponse._({this.currentVersion, required this.items})
    : super._();
  @override
  AgentReleaseListResponse rebuild(
    void Function(AgentReleaseListResponseBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  AgentReleaseListResponseBuilder toBuilder() =>
      AgentReleaseListResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is AgentReleaseListResponse &&
        currentVersion == other.currentVersion &&
        items == other.items;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, currentVersion.hashCode);
    _$hash = $jc(_$hash, items.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'AgentReleaseListResponse')
          ..add('currentVersion', currentVersion)
          ..add('items', items))
        .toString();
  }
}

class AgentReleaseListResponseBuilder
    implements
        Builder<AgentReleaseListResponse, AgentReleaseListResponseBuilder> {
  _$AgentReleaseListResponse? _$v;

  String? _currentVersion;
  String? get currentVersion => _$this._currentVersion;
  set currentVersion(String? currentVersion) =>
      _$this._currentVersion = currentVersion;

  ListBuilder<AgentReleaseResponse>? _items;
  ListBuilder<AgentReleaseResponse> get items =>
      _$this._items ??= ListBuilder<AgentReleaseResponse>();
  set items(ListBuilder<AgentReleaseResponse>? items) => _$this._items = items;

  AgentReleaseListResponseBuilder() {
    AgentReleaseListResponse._defaults(this);
  }

  AgentReleaseListResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _currentVersion = $v.currentVersion;
      _items = $v.items.toBuilder();
      _$v = null;
    }
    return this;
  }

  @override
  void replace(AgentReleaseListResponse other) {
    _$v = other as _$AgentReleaseListResponse;
  }

  @override
  void update(void Function(AgentReleaseListResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  AgentReleaseListResponse build() => _build();

  _$AgentReleaseListResponse _build() {
    _$AgentReleaseListResponse _$result;
    try {
      _$result =
          _$v ??
          _$AgentReleaseListResponse._(
            currentVersion: currentVersion,
            items: items.build(),
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'items';
        items.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
          r'AgentReleaseListResponse',
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
