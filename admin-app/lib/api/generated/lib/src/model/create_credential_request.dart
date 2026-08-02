//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'create_credential_request.g.dart';

/// CreateCredentialRequest
///
/// Properties:
/// * [name]
@BuiltValue()
abstract class CreateCredentialRequest implements Built<CreateCredentialRequest, CreateCredentialRequestBuilder> {
  @BuiltValueField(wireName: r'name')
  String get name;

  CreateCredentialRequest._();

  factory CreateCredentialRequest([void updates(CreateCredentialRequestBuilder b)]) = _$CreateCredentialRequest;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(CreateCredentialRequestBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<CreateCredentialRequest> get serializer => _$CreateCredentialRequestSerializer();
}

class _$CreateCredentialRequestSerializer implements PrimitiveSerializer<CreateCredentialRequest> {
  @override
  final Iterable<Type> types = const [CreateCredentialRequest, _$CreateCredentialRequest];

  @override
  final String wireName = r'CreateCredentialRequest';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    CreateCredentialRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'name';
    yield serializers.serialize(
      object.name,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    CreateCredentialRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required CreateCredentialRequestBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'name':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.name = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  CreateCredentialRequest deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = CreateCredentialRequestBuilder();
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
