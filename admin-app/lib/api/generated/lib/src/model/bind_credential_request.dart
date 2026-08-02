//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'bind_credential_request.g.dart';

/// BindCredentialRequest
///
/// Properties:
/// * [credentialId]
/// * [version]
@BuiltValue()
abstract class BindCredentialRequest implements Built<BindCredentialRequest, BindCredentialRequestBuilder> {
  @BuiltValueField(wireName: r'credential_id')
  String get credentialId;

  @BuiltValueField(wireName: r'version')
  int get version;

  BindCredentialRequest._();

  factory BindCredentialRequest([void updates(BindCredentialRequestBuilder b)]) = _$BindCredentialRequest;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(BindCredentialRequestBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<BindCredentialRequest> get serializer => _$BindCredentialRequestSerializer();
}

class _$BindCredentialRequestSerializer implements PrimitiveSerializer<BindCredentialRequest> {
  @override
  final Iterable<Type> types = const [BindCredentialRequest, _$BindCredentialRequest];

  @override
  final String wireName = r'BindCredentialRequest';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    BindCredentialRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'credential_id';
    yield serializers.serialize(
      object.credentialId,
      specifiedType: const FullType(String),
    );
    yield r'version';
    yield serializers.serialize(
      object.version,
      specifiedType: const FullType(int),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    BindCredentialRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required BindCredentialRequestBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'credential_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.credentialId = valueDes;
          break;
        case r'version':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(int),
          ) as int;
          result.version = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  BindCredentialRequest deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = BindCredentialRequestBuilder();
    final serializedList = (serialized as Iterable<Object?>).toList();
    final unhandled = <Object?>[];
    _deserializeProperties(
      serializers,
      serialized,
      specifiedType: specifiedType,
      serializedList: serializedList,
      unhandled: unhandled,
      result: result,
    );
    return result.build();
  }
}
