//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'create_git_credential_request.g.dart';

/// CreateGitCredentialRequest
///
/// Properties:
/// * [name]
@BuiltValue()
abstract class CreateGitCredentialRequest implements Built<CreateGitCredentialRequest, CreateGitCredentialRequestBuilder> {
  @BuiltValueField(wireName: r'name')
  String get name;

  CreateGitCredentialRequest._();

  factory CreateGitCredentialRequest([void updates(CreateGitCredentialRequestBuilder b)]) = _$CreateGitCredentialRequest;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(CreateGitCredentialRequestBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<CreateGitCredentialRequest> get serializer => _$CreateGitCredentialRequestSerializer();
}

class _$CreateGitCredentialRequestSerializer implements PrimitiveSerializer<CreateGitCredentialRequest> {
  @override
  final Iterable<Type> types = const [CreateGitCredentialRequest, _$CreateGitCredentialRequest];

  @override
  final String wireName = r'CreateGitCredentialRequest';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    CreateGitCredentialRequest object, {
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
    CreateGitCredentialRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required CreateGitCredentialRequestBuilder result,
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
  CreateGitCredentialRequest deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = CreateGitCredentialRequestBuilder();
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
