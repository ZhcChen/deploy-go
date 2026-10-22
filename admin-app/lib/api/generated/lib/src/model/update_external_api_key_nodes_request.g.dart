// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'update_external_api_key_nodes_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$UpdateExternalApiKeyNodesRequest
    extends UpdateExternalApiKeyNodesRequest {
  @override
  final BuiltList<String> nodeIds;

  factory _$UpdateExternalApiKeyNodesRequest([
    void Function(UpdateExternalApiKeyNodesRequestBuilder)? updates,
  ]) => (UpdateExternalApiKeyNodesRequestBuilder()..update(updates))._build();

  _$UpdateExternalApiKeyNodesRequest._({required this.nodeIds}) : super._();
  @override
  UpdateExternalApiKeyNodesRequest rebuild(
    void Function(UpdateExternalApiKeyNodesRequestBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  UpdateExternalApiKeyNodesRequestBuilder toBuilder() =>
      UpdateExternalApiKeyNodesRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is UpdateExternalApiKeyNodesRequest &&
        nodeIds == other.nodeIds;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, nodeIds.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(
      r'UpdateExternalApiKeyNodesRequest',
    )..add('nodeIds', nodeIds)).toString();
  }
}

class UpdateExternalApiKeyNodesRequestBuilder
    implements
        Builder<
          UpdateExternalApiKeyNodesRequest,
          UpdateExternalApiKeyNodesRequestBuilder
        > {
  _$UpdateExternalApiKeyNodesRequest? _$v;

  ListBuilder<String>? _nodeIds;
  ListBuilder<String> get nodeIds => _$this._nodeIds ??= ListBuilder<String>();
  set nodeIds(ListBuilder<String>? nodeIds) => _$this._nodeIds = nodeIds;

  UpdateExternalApiKeyNodesRequestBuilder() {
    UpdateExternalApiKeyNodesRequest._defaults(this);
  }

  UpdateExternalApiKeyNodesRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _nodeIds = $v.nodeIds.toBuilder();
      _$v = null;
    }
    return this;
  }

  @override
  void replace(UpdateExternalApiKeyNodesRequest other) {
    _$v = other as _$UpdateExternalApiKeyNodesRequest;
  }

  @override
  void update(void Function(UpdateExternalApiKeyNodesRequestBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  UpdateExternalApiKeyNodesRequest build() => _build();

  _$UpdateExternalApiKeyNodesRequest _build() {
    _$UpdateExternalApiKeyNodesRequest _$result;
    try {
      _$result =
          _$v ?? _$UpdateExternalApiKeyNodesRequest._(nodeIds: nodeIds.build());
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'nodeIds';
        nodeIds.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
          r'UpdateExternalApiKeyNodesRequest',
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
